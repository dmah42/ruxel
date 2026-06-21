pub const PROMPT_GLYPH: &str = "~ ";

#[derive(Debug, PartialEq, Clone, Copy)]
pub(crate) enum TimeOfDay {
    Morning,
    Day,
    Evening,
    Night,
}

impl TimeOfDay {
    pub fn to_radians(self) -> f32 {
        match self {
            TimeOfDay::Morning => 0.5,
            TimeOfDay::Day => std::f32::consts::FRAC_PI_2,
            TimeOfDay::Evening => 2.6,
            TimeOfDay::Night => -std::f32::consts::FRAC_PI_2,
        }
    }
}

impl std::str::FromStr for TimeOfDay {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "morning" => Ok(TimeOfDay::Morning),
            "day" | "noon" => Ok(TimeOfDay::Day),
            "evening" => Ok(TimeOfDay::Evening),
            "night" | "midnight" => Ok(TimeOfDay::Night),
            _ => Err(format!(
                "Invalid time '{}'. Expected: morning, day, evening, or night.",
                s
            )),
        }
    }
}

impl std::fmt::Display for TimeOfDay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TimeOfDay::Morning => "morning",
            TimeOfDay::Day => "day",
            TimeOfDay::Evening => "evening",
            TimeOfDay::Night => "night",
        };
        write!(f, "{}", s)
    }
}
#[derive(Debug, PartialEq)]
pub(crate) enum Command {
    Teleport(f32, f32, f32),
    Time(Option<TimeOfDay>),
    FindBiome(String),
    Help(Option<String>),
    Unknown(String),
    Error(String),
}

pub(crate) struct Console {
    is_open: bool,
    input: String,
    history: Vec<String>,
    pending_command: Option<Command>,
}

impl Console {
    pub(crate) fn new() -> Self {
        Self {
            is_open: false,
            input: String::new(),
            history: Vec::new(),
            pending_command: None,
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        self.is_open
    }

    pub(crate) fn toggle(&mut self) {
        self.is_open = !self.is_open;
    }

    pub(crate) fn input(&self) -> &str {
        &self.input
    }

    pub(crate) fn history(&self) -> &[String] {
        &self.history
    }

    pub(crate) fn handle_char(&mut self, c: char) {
        // Only accept printable ASCII for now to avoid weird control characters
        // Also ignore backquote/tilde to prevent it from entering the buffer when toggling
        if c.is_ascii() && !c.is_control() && c != '`' && c != '~' {
            self.input.push(c);
        }
    }

    pub(crate) fn handle_backspace(&mut self) {
        self.input.pop();
    }

    pub(crate) fn handle_enter(&mut self) {
        if self.input.trim().is_empty() {
            return;
        }

        let raw_cmd = self.input.trim().to_string();
        self.history.push(format!("{}{}", PROMPT_GLYPH, raw_cmd));

        self.pending_command = Some(Self::parse_command(&raw_cmd));
        self.input.clear();
    }

    pub(crate) fn push_history(&mut self, msg: String) {
        if !msg.is_empty() {
            for line in msg.lines() {
                self.history.push(line.to_string());
            }
        }
    }

    pub(crate) fn take_command(&mut self) -> Option<Command> {
        self.pending_command.take()
    }

    // TODO: better response when the args are wrong vs command is unknown.
    fn parse_command(input: &str) -> Command {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return Command::Unknown(input.to_string());
        }

        match parts[0] {
            "tp" | "teleport" => {
                if parts.len() == 4 {
                    if let (Ok(x), Ok(y), Ok(z)) = (
                        parts[1].parse::<f32>(),
                        parts[2].parse::<f32>(),
                        parts[3].parse::<f32>(),
                    ) {
                        return Command::Teleport(x, y, z);
                    } else {
                        return Command::Error(
                            "Invalid arguments for teleport. Expected 3 numbers, e.g. 'tp 0 100 0'"
                                .to_string(),
                        );
                    }
                }
                Command::Error("Invalid usage of teleport. Usage: teleport <x> <y> <z>".to_string())
            }
            "t" | "time" => {
                if parts.len() == 2 {
                    match parts[1].parse::<TimeOfDay>() {
                        Ok(t_val) => return Command::Time(Some(t_val)),
                        Err(e) => return Command::Error(e),
                    }
                } else if parts.len() == 1 {
                    return Command::Time(None);
                }
                Command::Error(
                    "Invalid usage of time. Usage: time [morning|day|evening|night]".to_string(),
                )
            }
            "fb" | "find_biome" => {
                if parts.len() == 2 {
                    return Command::FindBiome(parts[1].to_string());
                }
                Command::Error("Invalid usage of find_biome. Usage: find_biome <biome>".to_string())
            }
            "help" => {
                if parts.len() == 2 {
                    return Command::Help(Some(parts[1].to_string()));
                } else if parts.len() == 1 {
                    return Command::Help(None);
                }
                Command::Error("Invalid usage of help. Usage: help [command]".to_string())
            }
            _ => Command::Unknown(input.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_teleport() {
        assert_eq!(
            Console::parse_command("tp 1.0 2.5 -3.0"),
            Command::Teleport(1.0, 2.5, -3.0)
        );
        assert_eq!(
            Console::parse_command("teleport 0 100 0"),
            Command::Teleport(0.0, 100.0, 0.0)
        );
        assert_eq!(
            Console::parse_command("tp 1 2"),
            Command::Error("Invalid usage of teleport. Usage: teleport <x> <y> <z>".to_string())
        );
        assert_eq!(
            Console::parse_command("tp a b c"),
            Command::Error(
                "Invalid arguments for teleport. Expected 3 numbers, e.g. 'tp 0 100 0'".to_string()
            )
        );
    }

    #[test]
    fn test_parse_time() {
        assert_eq!(Console::parse_command("time"), Command::Time(None));
        assert_eq!(
            Console::parse_command("time morning"),
            Command::Time(Some(TimeOfDay::Morning))
        );
        assert_eq!(
            Console::parse_command("time night"),
            Command::Time(Some(TimeOfDay::Night))
        );
        assert_eq!(
            Console::parse_command("time 1.5"),
            Command::Error(
                "Invalid time '1.5'. Expected: morning, day, evening, or night.".to_string()
            )
        );
        assert_eq!(
            Console::parse_command("time foo"),
            Command::Error(
                "Invalid time 'foo'. Expected: morning, day, evening, or night.".to_string()
            )
        );
    }

    #[test]
    fn test_parse_find_biome() {
        assert_eq!(
            Console::parse_command("fb desert"),
            Command::FindBiome("desert".to_string())
        );
        assert_eq!(
            Console::parse_command("find_biome plains"),
            Command::FindBiome("plains".to_string())
        );
    }

    #[test]
    fn test_parse_help() {
        assert_eq!(Console::parse_command("help"), Command::Help(None));
        assert_eq!(
            Console::parse_command("help tp"),
            Command::Help(Some("tp".to_string()))
        );
        assert_eq!(
            Console::parse_command("help foo bar"),
            Command::Error("Invalid usage of help. Usage: help [command]".to_string())
        );
    }
}
