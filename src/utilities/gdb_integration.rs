
use std::process::{Command, Stdio};
use std::io::Write;

pub struct GdbSession {
    binary: String,
    breakpoints: Vec<Breakpoint>,
    commands: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Breakpoint {
    pub file: String,
    pub line: u32,
    pub enabled: bool,
    pub id: usize,
}

#[derive(Debug, Clone)]
pub struct WatchPoint {
    pub variable: String,
    pub condition: Option<String>,
    pub id: usize,
}

pub struct DebugContext {
    pub frame: u32,
    pub pc: u64,
    pub variables: std::collections::HashMap<String, String>,
}

impl GdbSession {
    pub fn new(binary: &str) -> Self {
        GdbSession {
            binary: binary.to_string(),
            breakpoints: Vec::new(),
            commands: Vec::new(),
        }
    }

    pub fn add_breakpoint(&mut self, file: &str, line: u32) -> usize {
        let id = self.breakpoints.len();
        self.breakpoints.push(Breakpoint {
            file: file.to_string(),
            line,
            enabled: true,
            id,
        });

        self.commands.push(format!("break {}:{}", file, line));
        id
    }

    pub fn remove_breakpoint(&mut self, id: usize) -> Result<(), String> {
        for bp in &mut self.breakpoints {
            if bp.id == id {
                bp.enabled = false;
                self.commands.push(format!("delete {}", id));
                return Ok(());
            }
        }
        Err(format!("Breakpoint {} not found", id))
    }

    pub fn enable_breakpoint(&mut self, id: usize) -> Result<(), String> {
        for bp in &mut self.breakpoints {
            if bp.id == id {
                bp.enabled = true;
                self.commands.push(format!("enable {}", id));
                return Ok(());
            }
        }
        Err(format!("Breakpoint {} not found", id))
    }

    pub fn disable_breakpoint(&mut self, id: usize) -> Result<(), String> {
        for bp in &mut self.breakpoints {
            if bp.id == id {
                bp.enabled = false;
                self.commands.push(format!("disable {}", id));
                return Ok(());
            }
        }
        Err(format!("Breakpoint {} not found", id))
    }

    pub fn add_command(&mut self, cmd: &str) {
        self.commands.push(cmd.to_string());
    }

    pub fn run(&self) -> Result<String, String> {
        let mut gdb_script = String::new();

        gdb_script.push_str("file ");
        gdb_script.push_str(&self.binary);
        gdb_script.push('\n');

        for cmd in &self.commands {
            gdb_script.push_str(cmd);
            gdb_script.push('\n');
        }

        gdb_script.push_str("quit\n");

        let output = Command::new("gdb")
            .arg("-batch")
            .arg("-ex")
            .arg(&gdb_script)
            .output()
            .map_err(|e| format!("Failed to run gdb: {}", e))?;

        String::from_utf8(output.stdout)
            .map_err(|e| format!("Failed to parse gdb output: {}", e))
    }

    pub fn run_interactive(&self) -> Result<(), String> {
        let mut gdb_script = String::new();

        gdb_script.push_str("file ");
        gdb_script.push_str(&self.binary);
        gdb_script.push('\n');

        for cmd in &self.commands {
            gdb_script.push_str(cmd);
            gdb_script.push('\n');
        }

        let mut child = Command::new("gdb")
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn gdb: {}", e))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(gdb_script.as_bytes())
                .map_err(|e| format!("Failed to write to gdb stdin: {}", e))?;
        }

        child.wait()
            .map_err(|e| format!("gdb failed: {}", e))?;

        Ok(())
    }

    pub fn set_breakpoint_condition(&mut self, id: usize, condition: &str) -> Result<(), String> {
        self.commands.push(format!("condition {} {}", id, condition));
        Ok(())
    }

    pub fn execute_with_breakpoints(&self, input_file: &str) -> Result<DebugContext, String> {
        let mut cmd_list = String::new();

        cmd_list.push_str("file ");
        cmd_list.push_str(&self.binary);
        cmd_list.push('\n');

        for bp in &self.breakpoints {
            if bp.enabled {
                cmd_list.push_str(&format!("break {}:{}\n", bp.file, bp.line));
            }
        }

        cmd_list.push_str("run < ");
        cmd_list.push_str(input_file);
        cmd_list.push('\n');

        cmd_list.push_str("info frame\n");
        cmd_list.push_str("info locals\n");

        let mut child = Command::new("gdb")
            .arg("-batch")
            .arg("-x")
            .arg("-")
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn gdb: {}", e))?;

        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(cmd_list.as_bytes());
        }

        Ok(DebugContext {
            frame: 0,
            pc: 0,
            variables: std::collections::HashMap::new(),
        })
    }

    pub fn get_backtrace(&self) -> Result<Vec<String>, String> {
        let mut script = String::new();
        script.push_str("file ");
        script.push_str(&self.binary);
        script.push_str("\nbacktrace\nquit\n");

        let output = Command::new("gdb")
            .arg("-batch")
            .arg("-ex")
            .arg(&script)
            .output()
            .map_err(|e| format!("Failed to run gdb: {}", e))?;

        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| format!("Failed to parse gdb output: {}", e))?;

        Ok(stdout.lines()
            .filter(|line| !line.is_empty())
            .map(|s| s.to_string())
            .collect())
    }

    pub fn print_breakpoints(&self) -> String {
        let mut output = String::new();
        output.push_str("Breakpoints:\n");
        for bp in &self.breakpoints {
            let status = if bp.enabled { "enabled" } else { "disabled" };
            output.push_str(&format!(
                "  [{}] {}:{} ({})\n",
                bp.id, bp.file, bp.line, status
            ));
        }
        output
    }

    pub fn get_commands_count(&self) -> usize {
        self.commands.len()
    }

    pub fn get_breakpoints_count(&self) -> usize {
        self.breakpoints.len()
    }

    pub fn clear(&mut self) {
        self.breakpoints.clear();
        self.commands.clear();
    }
}

pub struct DebuggerControlFlow {
    pub steps: Vec<String>,
    pub current_step: usize,
}

impl DebuggerControlFlow {
    pub fn new() -> Self {
        DebuggerControlFlow {
            steps: Vec::new(),
            current_step: 0,
        }
    }

    pub fn add_step(&mut self, description: &str) {
        self.steps.push(description.to_string());
    }

    pub fn next_step(&mut self) -> Option<&str> {
        if self.current_step < self.steps.len() {
            let step = &self.steps[self.current_step];
            self.current_step += 1;
            Some(step)
        } else {
            None
        }
    }

    pub fn reset(&mut self) {
        self.current_step = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gdb_session_creation() {
        let _session = GdbSession::new("./test");
    }

    #[test]
    fn test_add_breakpoint() {
        let mut session = GdbSession::new("./test");
        let id = session.add_breakpoint("main.rs", 10);
        assert_eq!(id, 0);
        assert_eq!(session.get_breakpoints_count(), 1);
    }

    #[test]
    fn test_remove_breakpoint() {
        let mut session = GdbSession::new("./test");
        let id = session.add_breakpoint("main.rs", 10);
        assert!(session.remove_breakpoint(id).is_ok());
    }

    #[test]
    fn test_enable_disable_breakpoint() {
        let mut session = GdbSession::new("./test");
        let id = session.add_breakpoint("main.rs", 10);
        assert!(session.disable_breakpoint(id).is_ok());
        assert!(session.enable_breakpoint(id).is_ok());
    }

    #[test]
    fn test_add_command() {
        let mut session = GdbSession::new("./test");
        session.add_command("run");
        assert_eq!(session.get_commands_count(), 1);
    }

    #[test]
    fn test_print_breakpoints() {
        let mut session = GdbSession::new("./test");
        session.add_breakpoint("main.rs", 10);
        let output = session.print_breakpoints();
        assert!(output.contains("Breakpoints:"));
    }

    #[test]
    fn test_debugger_control_flow() {
        let mut flow = DebuggerControlFlow::new();
        flow.add_step("Step 1");
        flow.add_step("Step 2");

        assert_eq!(flow.next_step(), Some("Step 1"));
        assert_eq!(flow.next_step(), Some("Step 2"));
        assert_eq!(flow.next_step(), None);
    }

    #[test]
    fn test_control_flow_reset() {
        let mut flow = DebuggerControlFlow::new();
        flow.add_step("Step 1");
        let _ = flow.next_step();
        flow.reset();
        assert_eq!(flow.next_step(), Some("Step 1"));
    }
}
