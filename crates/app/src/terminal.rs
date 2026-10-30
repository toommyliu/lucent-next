use lucent_runtime::ScriptControls;
use std::io::{self, BufRead};

pub fn spawn(controls: ScriptControls) -> io::Result<()> {
    std::thread::Builder::new()
        .name("lucent-terminal".into())
        .spawn(move || {
            tracing::info!("type stop, kill or status, then press Enter");
            for line in io::stdin().lock().lines() {
                let line = match line {
                    Ok(line) => line,
                    Err(error) => {
                        tracing::warn!(%error, "couldn't read terminal input");
                        break;
                    }
                };
                match line.trim() {
                    "stop" => controls.stop(),
                    "kill" => controls.kill(),
                    "status" => controls.status(),
                    "" => {}
                    line => tracing::warn!("unknown command {line:?}; type stop, kill or status"),
                }
            }
        })?;
    Ok(())
}
