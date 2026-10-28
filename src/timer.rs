//! Lógica pura do pomodoro (técnica de foco), sem nada de UI. Fácil de testar e reaproveitar.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Focus,
    ShortBreak,
    LongBreak,
}

impl Phase {
    pub fn label(self) -> &'static str {
        match self {
            Phase::Focus => "Foco",
            Phase::ShortBreak => "Pausa",
            Phase::LongBreak => "Pausa longa",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub focus_min: u32,
    pub short_break_min: u32,
    pub long_break_min: u32,
    /// Quantos focos até a pausa longa.
    pub cycles_before_long: u32,
    /// Começa a próxima fase automaticamente ao terminar a atual.
    pub auto_start: bool,
    pub sound: bool,
    /// Arquivo de som personalizado. Vazio usa o som do sistema.
    pub sound_file: String,
    pub always_on_top: bool,
    /// Mostra a música tocando no sistema.
    pub music: bool,
    /// Esconde o painel de música quando nada está tocando.
    pub music_only_when_playing: bool,
    pub lyrics: bool,
    /// Adianta (+) ou atrasa (−) a letra, em segundos.
    pub lyrics_offset: f32,
    /// Meta de focos por dia (Histórico).
    pub daily_goal: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            focus_min: 25,
            short_break_min: 5,
            long_break_min: 15,
            cycles_before_long: 4,
            auto_start: false,
            sound: true,
            sound_file: String::new(),
            always_on_top: true,
            music: true,
            music_only_when_playing: true,
            lyrics: true,
            lyrics_offset: 0.0,
            daily_goal: 8,
        }
    }
}

impl Settings {
    pub fn duration(&self, phase: Phase) -> Duration {
        let min = match phase {
            Phase::Focus => self.focus_min,
            Phase::ShortBreak => self.short_break_min,
            Phase::LongBreak => self.long_break_min,
        };
        Duration::from_secs(u64::from(min.max(1)) * 60)
    }
}

pub struct Timer {
    pub phase: Phase,
    /// Focos concluídos no ciclo atual.
    pub completed: u32,
    /// Tempo restante enquanto pausado.
    remaining: Duration,
    /// Instante em que a fase termina, se estiver rodando.
    deadline: Option<Instant>,
}

impl Timer {
    pub fn new(settings: &Settings) -> Self {
        Self {
            phase: Phase::Focus,
            completed: 0,
            remaining: settings.duration(Phase::Focus),
            deadline: None,
        }
    }

    pub fn is_running(&self) -> bool {
        self.deadline.is_some()
    }

    pub fn remaining(&self, now: Instant) -> Duration {
        match self.deadline {
            Some(d) => d.saturating_duration_since(now),
            None => self.remaining,
        }
    }

    /// Fração já percorrida da fase atual (0.0 a 1.0).
    pub fn progress(&self, settings: &Settings, now: Instant) -> f32 {
        let total = settings.duration(self.phase).as_secs_f32();
        1.0 - self.remaining(now).as_secs_f32() / total
    }

    pub fn start(&mut self, now: Instant) {
        if self.deadline.is_none() {
            self.deadline = Some(now + self.remaining);
        }
    }

    pub fn pause(&mut self, now: Instant) {
        if let Some(d) = self.deadline.take() {
            self.remaining = d.saturating_duration_since(now);
        }
    }

    pub fn toggle(&mut self, now: Instant) {
        if self.is_running() {
            self.pause(now)
        } else {
            self.start(now)
        }
    }

    /// Volta a fase atual ao início, parada.
    pub fn reset(&mut self, settings: &Settings) {
        self.deadline = None;
        self.remaining = settings.duration(self.phase);
    }

    /// Pula para a próxima fase.
    pub fn skip(&mut self, settings: &Settings, now: Instant) {
        self.advance(settings, now, false);
    }

    /// Deve ser chamado a cada quadro. Retorna a fase que acabou de terminar, se houver.
    pub fn tick(&mut self, settings: &Settings, now: Instant) -> Option<Phase> {
        let d = self.deadline?;
        if now < d {
            return None;
        }
        let finished = self.phase;
        self.advance(settings, d, settings.auto_start);
        Some(finished)
    }

    fn advance(&mut self, settings: &Settings, now: Instant, keep_running: bool) {
        self.phase = match self.phase {
            Phase::Focus => {
                self.completed += 1;
                if self.completed >= settings.cycles_before_long.max(1) {
                    self.completed = 0;
                    Phase::LongBreak
                } else {
                    Phase::ShortBreak
                }
            }
            Phase::ShortBreak | Phase::LongBreak => Phase::Focus,
        };
        self.remaining = settings.duration(self.phase);
        self.deadline = keep_running.then(|| now + self.remaining);
    }
}

pub fn format_mmss(d: Duration) -> String {
    // Arredonda para cima: mostra 25:00 no início e 00:01 no último segundo.
    let secs = d.as_secs() + u64::from(d.subsec_nanos() > 0);
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quick() -> Settings {
        Settings { focus_min: 1, short_break_min: 1, long_break_min: 2, cycles_before_long: 2, ..Default::default() }
    }

    #[test]
    fn pause_keeps_remaining() {
        let s = quick();
        let t0 = Instant::now();
        let mut t = Timer::new(&s);
        t.start(t0);
        t.pause(t0 + Duration::from_secs(20));
        assert_eq!(t.remaining(t0 + Duration::from_secs(999)), Duration::from_secs(40));
    }

    #[test]
    fn cycles_to_long_break() {
        let s = quick();
        let mut now = Instant::now();
        let mut t = Timer::new(&s);
        let mut seen = vec![];
        for _ in 0..4 {
            t.start(now);
            now += t.remaining(now);
            seen.push(t.tick(&s, now).unwrap());
        }
        assert_eq!(seen, [Phase::Focus, Phase::ShortBreak, Phase::Focus, Phase::LongBreak]);
        assert_eq!(t.phase, Phase::Focus);
    }

    #[test]
    fn auto_start_keeps_running() {
        let s = Settings { auto_start: true, ..quick() };
        let now = Instant::now();
        let mut t = Timer::new(&s);
        t.start(now);
        t.tick(&s, now + Duration::from_secs(60));
        assert!(t.is_running());
        assert_eq!(t.phase, Phase::ShortBreak);
    }

    #[test]
    fn mmss() {
        assert_eq!(format_mmss(Duration::from_secs(1500)), "25:00");
        assert_eq!(format_mmss(Duration::from_millis(300)), "00:01");
    }
}
