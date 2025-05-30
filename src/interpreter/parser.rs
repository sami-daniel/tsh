use std::{path::PathBuf, vec};

use anyhow::{Result, anyhow};

use crate::utils::report_line_err;

#[derive(Debug)]
pub enum Command {
    Simple {
        command_name: String,
        args: Vec<String>,
        redirects: Vec<Redirect>,
    },
}

#[derive(Debug)]
pub enum RedirectionType {
    Output,
    AppendOutput,
    RedirectToFileDescriptor(i32),
}

#[derive(Debug)]
pub enum RedirectionTarget {
    RealFile(PathBuf),
    FileDescriptor(i32),
}

#[derive(Debug)]
pub struct Redirect {
    pub from_fd: i32,
    pub kind: RedirectionType,
    pub target: RedirectionTarget,
}

pub fn try_parse_input(input: &str) -> Result<Option<Command>> {
    let mut chars = input.trim().chars().peekable();
    let mut single_quotes = false;
    let mut double_quotes = false;
    let mut arg_buffer = String::new();
    let mut args = vec![];
    let mut redirects = vec![];
    let mut current_redirect: Option<(i32, RedirectionType)> = None;

    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                if !double_quotes {
                    single_quotes = !single_quotes;
                } else {
                    arg_buffer.push(c);
                }
            }
            '"' => {
                if !single_quotes {
                    double_quotes = !double_quotes;
                } else {
                    arg_buffer.push(c);
                }
            }
            ' ' if !single_quotes && !double_quotes => {
                if !arg_buffer.is_empty() {
                    args.push(arg_buffer.clone());
                    arg_buffer.clear();
                }
            }
            '>' if !single_quotes && !double_quotes => {
                let mode = if chars.peek() == Some(&'>') {
                    chars.next();
                    RedirectionType::AppendOutput
                } else if chars.peek() == Some(&'@') {
                    // >@[fd_num]
                    chars.next();
                    let mut fd_num = String::with_capacity(1);
                    for fd_part in chars.by_ref() {
                        if !fd_part.is_ascii_digit() {
                            break;
                        }
                        fd_num.push(fd_part);
                    }

                    if fd_num.is_empty() {
                        return Err(anyhow!(
                            "Invalid number for redirect to file descriptor: [fd_to_redirect?1]>@[fd_to_receive_redirect]"
                        ));
                    }

                    if let Ok(fd_num) = fd_num.parse() {
                        RedirectionType::RedirectToFileDescriptor(fd_num)
                    } else {
                        report_line_err(Some(format!("Failed to parse 'redirect to' number file descriptor: Tried parse {}", fd_num).as_str()));
                        panic!()
                    }
                } else if chars.peek().is_none() {
                    return Err(anyhow!(
                        "Unexpected redirect token after '>': [fd_to_redirect?1]>[file]"
                    ));
                } else {
                    RedirectionType::Output
                };

                if let Ok(num) = arg_buffer.parse::<i32>() {
                    match mode {
                        RedirectionType::RedirectToFileDescriptor(fd) => {
                            redirects.push(Redirect {
                                from_fd: num,
                                kind: mode,
                                target: RedirectionTarget::FileDescriptor(fd),
                            });
                        }
                        _ => {
                            current_redirect = Some((num, mode));
                        }
                    }
                    arg_buffer.clear();
                } else {
                    current_redirect = Some((1, mode));
                }
            }
            _ => {
                if let Some((fd, mode)) = current_redirect {
                    match mode {
                        RedirectionType::Output | RedirectionType::AppendOutput => {
                            let mut single_quotes = false;
                            let mut double_quotes = false;
                            let mut path = String::from(c);
                            for arg_redirect_c in chars.by_ref() {
                                match arg_redirect_c {
                                    ' ' if !single_quotes && !double_quotes => {
                                        break;
                                    }
                                    '\'' => {
                                        if !double_quotes {
                                            single_quotes = !single_quotes;
                                        }
                                    }
                                    '"' => {
                                        if !single_quotes {
                                            double_quotes = !double_quotes;
                                        }
                                    }
                                    _ => path.push(arg_redirect_c),
                                }
                            }
                            redirects.push(Redirect {
                                from_fd: fd,
                                kind: mode,
                                target: RedirectionTarget::RealFile(PathBuf::from(path)),
                            });
                        }
                        _ => {
                            // do not rely on having a next character to process the file redirect,
                            // that's why we process it as soon as we detect a redirect.
                        }
                    }

                    current_redirect = None;
                } else {
                    arg_buffer.push(c);
                }
            }
        }
    }

    if !arg_buffer.is_empty() {
        args.push(arg_buffer);
    }

    if let Some(name) = args.first() {
        let name = name.trim().to_string();

        Ok(Some(Command::Simple {
            command_name: name,
            args: args.into_iter().skip(1).collect(),
            redirects,
        }))
    } else {
        Ok(None)
    }
}
