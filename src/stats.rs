//! Histórico simples: quantos focos foram concluídos por dia.

use chrono::{Days, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Day {
    pub sessions: u32,
    pub minutes: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Stats {
    pub days: BTreeMap<NaiveDate, Day>,
}

pub fn today() -> NaiveDate {
    chrono::Local::now().date_naive()
}

impl Stats {
    pub fn record(&mut self, date: NaiveDate, minutes: u32) {
        let d = self.days.entry(date).or_default();
        d.sessions += 1;
        d.minutes += minutes;
    }

    pub fn day(&self, date: NaiveDate) -> Day {
        self.days.get(&date).copied().unwrap_or_default()
    }

    /// Dias seguidos com pelo menos um foco, terminando hoje (ou ontem, se hoje ainda não teve).
    pub fn streak(&self, today: NaiveDate) -> u32 {
        let mut date = if self.day(today).sessions > 0 { today } else { today - Days::new(1) };
        let mut n = 0;
        while self.day(date).sessions > 0 {
            n += 1;
            date = date - Days::new(1);
        }
        n
    }

    pub fn total(&self) -> Day {
        self.days.values().fold(Day::default(), |a, d| Day { sessions: a.sessions + d.sessions, minutes: a.minutes + d.minutes })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streak_counts_back_from_today_or_yesterday() {
        let d = |n| NaiveDate::from_ymd_opt(2026, 10, n).unwrap();
        let mut s = Stats::default();
        s.record(d(25), 25);
        s.record(d(26), 25);
        s.record(d(27), 25);
        assert_eq!(s.streak(d(27)), 3);
        assert_eq!(s.streak(d(28)), 3); // hoje ainda sem foco, a sequência continua viva
        assert_eq!(s.streak(d(29)), 0);
        assert_eq!(s.total(), Day { sessions: 3, minutes: 75 });
    }
}
