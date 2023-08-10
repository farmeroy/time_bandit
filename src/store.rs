use std::collections::HashMap;

use crate::types::types::{Event, Task};
use rusqlite::{params, Connection, OptionalExtension, Result};

#[derive(Debug)]
pub struct Store {
    connection: Connection,
}

fn get_task_id_by_name(conn: &Connection, task_name: &str) -> Result<Option<i32>> {
    let mut stmt = conn.prepare("SELECT id FROM task WHERE name = ?")?;
    let task_id: Result<Option<i32>> = stmt.query_row(&[task_name], |row| row.get(0));
    Ok(task_id.unwrap_or(None))
}

fn create_task(conn: &Connection, task_name: &str, details: &str) -> Result<Task> {
    conn.execute(
        "INSERT INTO task (name, details) VALUES (?1, ?2)",
        &[task_name, details],
    )?;
    let task_id = conn.last_insert_rowid() as i32;

    let new_task = Task {
        id: task_id,
        name: task_name.to_string(),
        details: Some(String::from("")),
        events: None,
    };

    Ok(new_task)
}

fn create_event(
    conn: &Connection,
    task_id: &i32,
    notes: &str,
    now: &str,
    duration: &str,
) -> Result<()> {
    let mut stmt = conn.prepare(
        "INSERT INTO event (task_id, notes, time_stamp, duration) VALUES (?1, ?2, ?3, ?4) ",
    )?;
    stmt.execute(params![task_id, notes, now, duration])?;
    Ok(())
}
// create an event type
// so that each task can have multiple events
// id -> task id
// duration
// timestamp

impl Store {
    pub fn new(db_url: &str) -> Result<Self> {
        let conn = Connection::open(db_url)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS task (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            details TEXT
    )",
            (),
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS event (
                id INTEGER PRIMARY KEY,
                task_id INTEGER,
                notes TEXT,
                time_stamp STRING,
                duration STRING,
                FOREIGN KEY(task_id) REFERENCES task(id)
            )",
            (),
        )?;
        Ok(Store { connection: conn })
    }
    pub fn add_task(
        &self,
        name: String,
        details: String,
        now: String,
        duration: String,
    ) -> Result<(), rusqlite::Error> {
        if let Some(task_id) = get_task_id_by_name(&self.connection, &name)? {
            create_event(&self.connection, &task_id, &details, &now, &duration)?;
        } else {
            let new_task = create_task(&self.connection, &name, &details)?;
            create_event(&self.connection, &new_task.id, &details, &now, &duration)?;
        }
        Ok(())
    }
    pub fn get_tasks(&self) -> Result<Vec<Task>> {
        let stmt = &mut self.connection.prepare(
            "
        SELECT
            task.id AS task_id,
            task.name AS task_name,
            task.details AS task_details,
            event.id AS event_id,
            event.task_id AS event_task_id,
            event.notes AS event_notes,
            event.time_stamp AS event_time_stamp,
            event.duration AS event_duration
        FROM
            task
        LEFT JOIN
            event ON task.id = event.task_id
    ",
        )?;

        let mut rows = stmt.query([])?;

        let mut tasks_map: HashMap<i32, (Task, Vec<Event>)> = HashMap::new();

        while let Some(row) = rows.next()? {
            let task_id: i32 = row.get("task_id")?;
            let task_name: String = row.get("task_name")?;
            let task_details: Option<String> = row.get("task_details")?;
            let event_id: Option<i32> = row.get("event_id")?;
            let event_task_id: Option<i32> = row.get("event_task_id")?;
            let event_notes: Option<String> = row.get("event_notes")?;
            let event_time_stamp: Option<String> = row.get("event_time_stamp")?;
            let event_duration: Option<String> = row.get("event_duration")?;

            let task = Task {
                id: task_id,
                name: task_name,
                details: task_details,
                events: None, // We'll populate this shortly
            };

            let event = match event_id {
                Some(id) => Event {
                    id,
                    task_id: event_task_id.unwrap(), // Safe unwrap since event_id is Some
                    notes: event_notes,
                    time_stamp: event_time_stamp.unwrap(), // Safe unwrap
                    duration: event_duration.unwrap(),     // Safe unwrap
                },
                None => continue, // No event for this task
            };

            let entry = tasks_map.entry(task_id).or_insert((task, Vec::new()));
            entry.1.push(event);
        }

        // Convert HashMap values into Vec<Task> with their associated events
        let tasks_with_events: Vec<Task> = tasks_map
            .values()
            .map(|(task, events)| Task {
                id: task.id,
                name: task.name.clone(),
                details: task.details.clone(),
                events: Some(events.clone().to_vec()),
            })
            .collect();

        Ok(tasks_with_events)
    }
    pub fn get_events(&self) -> Result<Vec<Event>> {
        let stmt = &mut self
            .connection
            .prepare("SELECT id, task_id, notes, time_stamp, duration FROM event")?;
        let event_iter = stmt.query_map([], |row| {
            Ok(Event {
                id: row.get(0)?,
                task_id: row.get(1)?,
                notes: row.get(2)?,
                time_stamp: row.get(3)?,
                duration: row.get(4)?,
            })
        })?;
        let mut events = vec![];
        for event in event_iter {
            let event = event.unwrap();
            events.push(event)
        }
        Ok(events)
    }
}
