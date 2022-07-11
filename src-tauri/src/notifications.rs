use async_cron_scheduler::{Job, JobId, Scheduler};
use chrono::offset::Local;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Mutex;
use tauri::api::notification::Notification;
use tauri::{command, State};

#[derive(Serialize, Deserialize)]
pub enum Repeat {
  #[serde(rename = "never")]
  Never,
  #[serde(rename = "daily")]
  Daily,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Group {
  pub title: String,
  pub description: String,
  pub enabled: bool,
  pub id: String,
  #[serde(skip)]
  pub job_id: Option<JobId>,
  pub cron: String,
  pub next_date: Option<u64>,
}
impl Group {
  pub fn create_job(&mut self, scheduler: &mut Scheduler<Local>, a: String) -> Result<(), String> {
    if self.enabled {
      let job = match Job::cron(self.cron.as_str()) {
        Ok(job) => job,
        Err(e) => return Err(e.to_string()),
      };
      let group = self.clone();
      let job_id = scheduler.insert(job, move |_id| {
        let result = Notification::new(&a)
          .title(&group.title)
          .body(&group.description)
          .show();
        match result {
          Ok(_) => {}
          Err(e) => eprintln!("Could not show notification: {}", e),
        }
      });
      self.job_id = Some(job_id);
      println!("Created job \"{}\" at {}", self.title, self.cron);
    }
    Ok(())
  }
}

pub struct Instance {
  pub scheduler: Option<Scheduler<Local>>,
  pub groups: Vec<Group>,
  pub bundle_identifier: String,
}
impl Instance {
  pub fn add_group(&mut self, mut group: Group) -> Result<(), String> {
    match &mut self.scheduler {
      Some(scheduler) => {
        group.create_job(scheduler, self.bundle_identifier.clone())?;
        self.groups.push(group);
      }
      None => {
        self.groups.push(group);
        self.start()?;
      }
    };
    Ok(())
  }
  pub fn generate_id(&self) -> String {
    let alphabet: [char; 32] = [
      'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
      's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '2', '3', '4', '5', '6', '7',
    ];
    for _ in 0..100 {
      let id = nanoid!(7, &alphabet);
      let exists = self.groups.iter().any(|g| g.id == id);
      if !exists {
        return id;
      }
    }
    panic!("Error generating ID: Generated IDs already exist")
  }
  pub fn delete_group(&mut self, index: usize) {
    let scheduler = match &mut self.scheduler {
      Some(scheduler) => scheduler,
      None => {
        self.groups.remove(index);
        return;
      }
    };
    match self.groups[index].job_id {
      Some(job_id) => scheduler.remove(job_id),
      None => {}
    };
    self.groups.remove(index);
  }
  pub fn start(&mut self) -> Result<(), String> {
    let (mut scheduler, sched_service) = Scheduler::<Local>::launch(tokio::time::sleep);

    let mut err = None;
    for group in &mut self.groups {
      match group.create_job(&mut scheduler, self.bundle_identifier.clone()) {
        Ok(_) => {}
        Err(e) => {
          err = Some(e);
        }
      };
    }
    self.scheduler = Some(scheduler);

    tokio::spawn(sched_service);

    match err {
      Some(e) => Err(e.to_string()),
      None => Ok(()),
    }
  }
}

pub struct Data(pub Mutex<Instance>);

pub fn to_json<T: Serialize>(data: &T) -> Result<Value, String> {
  match serde_json::to_value(data) {
    Ok(v) => Ok(v),
    Err(e) => throw!("Error serializing {}", e),
  }
}

#[command]
pub async fn get_groups(data: State<'_, Data>) -> Result<Value, String> {
  let data = data.0.lock().unwrap();
  to_json(&data.groups)
}

#[command]
pub async fn new_group(mut group: Group, data: State<'_, Data>) -> Result<Value, String> {
  let mut data = data.0.lock().unwrap();
  group.id = data.generate_id();
  data.add_group(group)?;
  to_json(&data.groups)
}

#[command]
pub async fn delete_group(index: usize, data: State<'_, Data>) -> Result<Value, String> {
  let mut data = data.0.lock().unwrap();
  data.delete_group(index);
  to_json(&data.groups)
}
