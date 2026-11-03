use super::memory::Memory;
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const BDA_TICKS: u32 = 0x46C;

pub struct DateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub weekday: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub hundredths: u8,
}

fn since_epoch() -> (u64, u32) {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    (elapsed.as_secs(), elapsed.subsec_millis())
}

pub fn now() -> DateTime {
    let (seconds, millis) = since_epoch();
    breakdown(seconds, millis)
}

pub fn of_system_time(time: SystemTime) -> DateTime {
    let elapsed = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    breakdown(elapsed.as_secs(), elapsed.subsec_millis())
}

impl DateTime {
    pub fn dos_date(&self) -> u16 {
        ((self.year.saturating_sub(1980)) << 9) | ((self.month as u16) << 5) | self.day as u16
    }

    pub fn dos_time(&self) -> u16 {
        ((self.hour as u16) << 11) | ((self.minute as u16) << 5) | (self.second as u16 / 2)
    }
}

fn breakdown(seconds: u64, millis: u32) -> DateTime {
    let days = (seconds / 86_400) as i64;
    let of_day = seconds % 86_400;
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 { month_index + 3 } else { month_index - 9 };
    let year = year_of_era + era * 400 + if month <= 2 { 1 } else { 0 };
    DateTime {
        year: year as u16,
        month: month as u8,
        day: day as u8,
        weekday: ((days + 4).rem_euclid(7)) as u8,
        hour: (of_day / 3_600) as u8,
        minute: (of_day / 60 % 60) as u8,
        second: (of_day % 60) as u8,
        hundredths: (millis / 10) as u8,
    }
}

struct TimeScale {
    speed: u32,
    real_anchor: Instant,
    virtual_anchor_ms: u64,
}

fn real_millis_of_day() -> u64 {
    let (seconds, millis) = since_epoch();
    (seconds % 86_400) * 1_000 + millis as u64
}

fn scale() -> &'static Mutex<TimeScale> {
    static SCALE: OnceLock<Mutex<TimeScale>> = OnceLock::new();
    SCALE.get_or_init(|| {
        Mutex::new(TimeScale { speed: 1, real_anchor: Instant::now(), virtual_anchor_ms: real_millis_of_day() })
    })
}

fn virtual_millis_of_day(state: &TimeScale) -> u64 {
    let elapsed = state.real_anchor.elapsed().as_millis() as u64;
    (state.virtual_anchor_ms + elapsed * state.speed as u64) % (86_400 * 1_000)
}

pub fn set_speed(speed: u32) {
    let Ok(mut state) = scale().lock() else {
        return;
    };
    state.virtual_anchor_ms = virtual_millis_of_day(&state);
    state.real_anchor = Instant::now();
    state.speed = speed.clamp(1, 16);
}

pub fn ticks_since_midnight() -> u32 {
    let millis_of_day = match scale().lock() {
        Ok(state) => virtual_millis_of_day(&state),
        Err(_) => real_millis_of_day(),
    };
    (millis_of_day * 1_193_182 / 65_536 / 1_000) as u32
}

pub fn store_ticks(memory: &mut Memory) {
    let ticks = ticks_since_midnight();
    memory.write16(BDA_TICKS, ticks as u16);
    memory.write16(BDA_TICKS + 2, (ticks >> 16) as u16);
}
