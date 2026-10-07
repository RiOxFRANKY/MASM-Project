use super::{Halt, Machine};
use crate::hardware::clock;
use crate::terminal::{Input, Key};
use std::time::Duration;

const WAIT_SLICE: Duration = Duration::from_millis(50);

impl Machine {
    pub(crate) fn pump(&mut self, timeout: Duration) -> Result<bool, Halt> {
        match self.terminal.poll(timeout) {
            Some(Input::Key(key)) => {
                self.keyboard.push(key);
                Ok(true)
            }
            Some(Input::Break) => Err(Halt::Break),
            Some(Input::Quit) => Err(Halt::Quit),
            None => Ok(false),
        }
    }

    pub fn wait_key(&mut self) -> Result<Key, Halt> {
        loop {
            if let Some(key) = self.keyboard.pop() {
                return Ok(key);
            }
            self.present();
            clock::store_ticks(&mut self.memory);
            self.pump(WAIT_SLICE)?;
        }
    }

    pub fn peek_key(&mut self) -> Result<Option<Key>, Halt> {
        if self.keyboard.is_empty() {
            self.pump(Duration::ZERO)?;
        }
        Ok(self.keyboard.peek())
    }

    pub(crate) fn idle(&mut self) -> Result<(), Halt> {
        self.present();
        clock::store_ticks(&mut self.memory);
        if self.keyboard.is_empty() {
            self.pump(Duration::from_millis(5))?;
        }
        Ok(())
    }
}
