use std::{env, fs};
use std::env::args;
use std::fs::File;
use std::io::{ErrorKind, Write};
use std::process::exit;
use console::{style, Style};
use regex::Regex;

fn description() {
    println!("{} {}",
             style("->").bold().green(),
             style("domake is a simple CLI tool that generates a Makefile\n\
      from a custom and simpler file named `Dofile`.").bold().blue())
}

fn usage() {
    let title_style = Style::new().bold().green();
    let text_style = Style::new().bold().cyan();
    println!("{}\n\
                {}\n\
              {}\n\
                {:18}{}\n\
                {:18}{}\n\
              {}\n\
                {}\n\
                {}",
             title_style.apply_to("Usage:"),
             text_style.apply_to("\tdomake [OPTION]"),
             title_style.apply_to("Options:"),
             text_style.apply_to("\t-h, --help"), text_style.apply_to("Prints help information"),
             text_style.apply_to("\t-v, --version"), text_style.apply_to("Prints version information"),
             title_style.apply_to("Conditions:"),
             text_style.apply_to("\t- you need to have a valid `Dofile` in the current directory."),
             text_style.apply_to("\t- any `Makefile` existent in the current directory will be erased after confirmation."));
}

fn version() {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    exit(0)
}

fn main() {
    let args = args().skip(1).collect::<Vec<_>>();
    if !args.is_empty() {
        if args.len() > 1 {
            error("Too many arguments");
        }
        match args.first().unwrap().as_str() {
            "-v" | "--version" => version(),
            "-h" | "--help" => help(),
            _ => error("Wrong argument"),
        }
    }

    if is_makefile() {
        let ok = confirm();
        if !ok { exit(0); }
    }

    let file = read_file();

    match file {
        Err(err) => {
            if err.kind() == ErrorKind::NotFound {
                println!("{} {}", style("No 'Dofile' found in directory").bold().red(), get_pwd());
            }
            error(err.to_string().as_str());
        },
        Ok(content) => {
            println!("{}", style("-> Dofile found").bold().green());
            let (includes, cmds) = parse(content);
            println!("{}", style("-> Content parsed").bold().green());

            let res = write((includes, cmds));
            match res {
                Ok(_) => {
                    println!("{}", style("-> Makefile successfully created!").bold().green());
                }
                Err(_) => {
                    println!("Error writing to file!");
                    exit(2);
                }
            }
        }
    }
    exit(0)
}

fn write(contents: (Vec<String>, Vec<Command>)) -> Result<(), std::io::Error> {
    let make_helpers = include_str!("../make_helpers");
    let (includes, cmds) = contents;
    let mut file = File::create("Makefile")?;

    let mut buffer: String = String::new();
    // add the header
    buffer.push_str("# This Makefile was done using 'domake'\n");
    buffer.push_str(format!("# Generated at {}\n", chrono::offset::Local::now().format("%d/%m/%Y")).as_str());
    buffer.push_str("\n");

    // add the includes
    for include in includes {
        buffer.push_str(format!("include {}\n", include).as_str());
    }
    buffer.push_str("\n");

    // add the helpers
    buffer.push_str(format!("{}\n", make_helpers).as_str());
    buffer.push_str("\n");

    // add the commands
    for cmd in cmds {
        buffer.push_str(format!("{}\n", cmd.to_makefile()).as_str());
    }

    file.write_all(buffer.as_bytes())?;
    Ok(())
}

struct Command {
    name: String,
    description: String,
    prior_commands: String,
    instructions: Vec<String>,
}

impl Command {
    fn to_makefile(&self) -> String {
        let mut buffer = format!(
            "## {}: {}\n\
            .PHONY: {}\n\
            {}: {}\n",
            self.name, self.description[1..].trim(),
            self.name,
            self.name, self.prior_commands);

        for instruction in &self.instructions {
            buffer.push_str(format!("\t{}\n", instruction).as_str());
        }
        buffer
    }
}

fn is_makefile() -> bool {
    fs::exists("Makefile").unwrap()
}

fn get_pwd() -> String {
    let path = env::current_dir();
    match path {
        Ok(path) => path.to_str().unwrap().to_string(),
        Err(_) => "NAN".to_string(),
    }
}

fn read_file() -> Result<String, std::io::Error> {
    std::fs::read_to_string("Dofile")
}

fn parse(content: String) -> (Vec<String>, Vec<Command>) {
    let re_includes = Regex::new(r"include (?<include>[[:print:]]+)").unwrap();

    let includes: Vec<String> = re_includes.captures_iter(&content).map(|c| {
        c.name("include").unwrap().as_str().to_string()
    }).collect::<Vec<String>>();

    let re_commands = Regex::new(r"(?<name>\[[[:print:]]+])(?:\r\n|\n)?(?<prior_commands>[[:print:]]+)?(?:\r\n|\n)(?<description>#[[:print:]]+)(?:\r\n|\n)(?<instructions>(?:[[:print:]]+(?:\r\n|\n)?)+)").unwrap();

    let commands: Vec<Command> = re_commands.captures_iter(&content).map(|c| {

        let name = c.name("name").unwrap().as_str().trim_start_matches("[").trim_end_matches("]").to_string();
        let prior_commands = c.name("prior_commands").map(|m| m.as_str().to_string()).unwrap_or_default();
        let description = c.name("description").unwrap().as_str().to_string();
        let all_instructions = c.name("instructions").unwrap().as_str().to_string();
        let instructions = all_instructions.split('\n').map(|i| i.to_string()).collect::<Vec<_>>();

        Command {
            name,
            prior_commands,
            description,
            instructions
        }
    }).collect::<Vec<Command>>();

    (includes, commands)
}

fn confirm() -> bool {
    let intro = style("A Makefile has been found in the current directory.\n\
        Do you want to overwrite it?").bold().yellow();
    let warning = style("(you will lose all data previously present in the Makefile)").bold().red();
    let options = style("> [y/N]").bold().blue();

    print!("{} {}\n{} ", intro, warning, options);
    let _ = std::io::stdout().flush();

    let mut choice = String::new();
    let res = std::io::stdin().read_line(&mut choice);
    if let Err(_) = res {
        error("Failed to read input from stdin");
        return false;
    }
    match choice.trim().to_lowercase().as_str() {
        "y" | "yes" => { true }
        _ => { false }
    }
}

fn error(err: &str) {
    println!("{} {}", style("Error:").bold().red(), style(err).red());
    println!();
    usage();
    exit(1);
}

fn help() {
    description();
    println!();
    usage();
    exit(0);
}
