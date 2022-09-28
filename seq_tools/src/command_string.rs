#[derive(Debug)]
pub enum CommandCategory{
    Header,
    Calculation,
    Constant,
    VarInit,
    Declaration,
    HardwareExec,
    LoopStart(i32),
    LoopEnd(i32),
}

#[derive(Debug)]
pub struct CommandString {
    pub commands:String,
    pub category:Option<CommandCategory>
}

pub trait Command {
    fn command_strings(&self) -> Vec<CommandString>;
}

impl CommandString{
    pub fn new(command:&str,category:CommandCategory) -> CommandString{
        CommandString{
            commands:command.to_owned(),
            category:Some(category)
        }
    }
    pub fn new_constant(command:&str) -> CommandString {
        CommandString::new(command,CommandCategory::Constant)
    }
    pub fn new_calculation(command:&str) -> CommandString {
        CommandString::new(command,CommandCategory::Calculation)
    }
    pub fn new_init(command:&str) -> CommandString {
        CommandString::new(command,CommandCategory::VarInit)
    }
    pub fn new_declare(command:&str) -> CommandString {
        CommandString::new(command,CommandCategory::Declaration)
    }
    pub fn new_header(command:&str) -> CommandString {
        CommandString::new(command,CommandCategory::Header)
    }
    pub fn new_hardware_exec(command:&str) -> CommandString {
        CommandString::new(command,CommandCategory::HardwareExec)
    }
    pub fn new_loop_start(command:&str,priority:i32) -> CommandString {
        CommandString::new(command,CommandCategory::LoopStart(priority))
    }
    pub fn new_loop_end(command:&str,priority:i32) -> CommandString {
        CommandString::new(command,CommandCategory::LoopEnd(priority))
    }
}