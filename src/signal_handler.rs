// ctrl C
// ctrl D
// up arrow
// down arrow
pub mod SignalHandler {
    use tokio::process::Child;
    use nix::sys::signal::{Signal};
    use nix::libc::{kill};
    use nix::unistd::Pid;

    pub async fn handle_ctrl_c(child: &mut Child) -> i32 {
        let child_id: i32 = child.id().expect("SignalHandler::handle_ctrl_c child process has no id") as i32;
        tokio::select! {
            status = child.wait() => {
                match status {
                    Ok(status) => status.code().unwrap_or(1),
                    Err(_) => panic!("SignalHandler::handle_ctrl_c issue while waiting for child"),
                }

            }
            _ = tokio::signal::ctrl_c() => {
                unsafe { kill(child_id, Signal::SIGINT as i32);}
                return 2;
            }
        }
    }
}
