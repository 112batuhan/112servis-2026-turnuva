//! Operations on the site's data that more than the HTTP routes run: the room bots save
//! scores and link rooms, the Discord bot answers notifications, and several places work
//! the potential matches out again. Each is one transaction that queues its Discord jobs
//! and tells the open pages; the handlers check who's asking before calling them.
pub mod bracket;
pub mod matches;
pub mod notifications;
pub mod seeding;
pub mod stats;
