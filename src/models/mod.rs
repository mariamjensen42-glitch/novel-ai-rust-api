pub mod character;
pub mod chapter;
pub mod novel;
pub mod outline;
pub mod project;
pub mod user;

use nanoid::nanoid;

pub fn new_id() -> String {
    nanoid!(21)
}
