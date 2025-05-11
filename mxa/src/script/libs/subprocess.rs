use std::time::Duration;

use log::{debug, error, info, trace, warn};
use mlua::{AnyUserData, UserData};
use tokio::{
  io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
  process::{Child, ChildStderr, ChildStdin, ChildStdout, Command},
};

struct SubprocessInstance {
  _command: Command,
  child: Child,
  stdin: Option<BufWriter<ChildStdin>>,
  stdout: BufReader<ChildStdout>,
  stderr: BufReader<ChildStderr>,
  exit_code: Option<i32>,
}

impl SubprocessInstance {
  async fn try_new(program: String, args: Vec<String>) -> anyhow::Result<Self> {
    info!("Spawning subprocess: {program} {args:?}");
    let mut command = Command::new(program);
    command.args(args);
    command.stdin(std::process::Stdio::piped());
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
    let mut child = command.spawn()?;
    trace!(
      "Subprocess spawned with PID: {}",
      child.id().ok_or(anyhow::anyhow!("Failed to get PID"))?
    );
    let Some(stdin) = child.stdin.take() else {
      warn!("Failed to get stdin");
      child.kill().await?;
      anyhow::bail!("Failed to get stdin");
    };
    let Some(stdout) = child.stdout.take() else {
      warn!("Failed to get stdout");
      child.kill().await?;
      anyhow::bail!("Failed to get stdout");
    };
    let Some(stderr) = child.stderr.take() else {
      warn!("Failed to get stderr");
      child.kill().await?;
      anyhow::bail!("Failed to get stderr");
    };
    let instance = SubprocessInstance {
      _command: command,
      child,
      stdin: Some(BufWriter::new(stdin)),
      stdout: BufReader::new(stdout),
      stderr: BufReader::new(stderr),
      exit_code: None,
    };
    Ok(instance)
  }

  async fn write_in(&mut self, data: String) -> anyhow::Result<()> {
    if let Some(stdin) = &mut self.stdin {
      trace!("Writing to stdin: {data}");
      stdin.write(data.as_bytes()).await?;
      stdin.flush().await?;
    }
    Ok(())
  }

  async fn read_out(&mut self) -> anyhow::Result<Option<String>> {
    let mut buf = String::new();
    if let Err(_) = tokio::time::timeout(Duration::from_secs(20), async {
      trace!("Reading from stdout with deadline");
      self.stdout.read_line(&mut buf).await
    })
    .await?
    {
      return Ok(None);
    }
    Ok(Some(buf))
  }

  async fn read_err(&mut self) -> anyhow::Result<Option<String>> {
    let mut buf = String::new();
    if let Err(_) = tokio::time::timeout(Duration::from_secs(20), async {
      trace!("Reading from stderr with deadline");
      self.stderr.read_line(&mut buf).await
    })
    .await?
    {
      return Ok(None);
    }
    Ok(Some(buf))
  }

  async fn close_stdin(&mut self) -> anyhow::Result<()> {
    if let Some(mut stdin) = self.stdin.take() {
      trace!("Closing stdin");
      stdin.shutdown().await?;
    }
    Ok(())
  }

  async fn read_out_to_end(&mut self) -> anyhow::Result<Option<String>> {
    let mut buf = String::new();
    if let Err(_) = tokio::time::timeout(Duration::from_secs(20), async {
      trace!("Reading from stdout to end with deadline");
      self.stdout.read_to_string(&mut buf).await
    })
    .await?
    {
      return Ok(None);
    }
    Ok(Some(buf))
  }

  async fn read_err_to_end(&mut self) -> anyhow::Result<Option<String>> {
    let mut buf = String::new();
    if let Err(_) = tokio::time::timeout(Duration::from_secs(20), async {
      trace!("Reading from stderr to end with deadline");
      self.stderr.read_to_string(&mut buf).await
    })
    .await?
    {
      return Ok(None);
    }
    Ok(Some(buf))
  }

  async fn wait(&mut self) -> anyhow::Result<i32> {
    if let Some(exit_code) = self.exit_code {
      trace!("Process already exited with code {exit_code}");
      return Ok(exit_code);
    }
    let status = self.child.wait().await?;
    let exit_code = match status.code() {
      Some(code) => {
        trace!("Process exited with code {code}");
        code
      }
      None => {
        error!("Process terminated by signal");
        return Ok(-1);
      }
    };
    self.exit_code = Some(exit_code);
    Ok(exit_code)
  }

  async fn terminate(&mut self) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
      if let Some(pid) = self.child.id() {
        let pid = pid as i32;
        if let Err(e) = nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), nix::sys::signal::Signal::SIGTERM) {
          error!("Failed to send SIGTERM to process {pid}: {e}");
        } else {
          let _ = tokio::time::timeout(Duration::from_secs(20), self.wait()).await;
        }
      } else {
        error!("Failed to get process ID");
      }
    }
    self.child.kill().await?;
    Ok(())
  }

  async fn kill(&mut self) -> anyhow::Result<()> {
    self.child.kill().await?;
    Ok(())
  }
}

struct Subprocess {
  _command: String,
  _args: Vec<String>,
  child: Option<SubprocessInstance>,
}

impl Subprocess {
  fn new(cmd: String, args: Vec<String>) -> Self {
    Subprocess {
      _command: cmd,
      _args: args,
      child: None,
    }
  }

  async fn spawn(&mut self) -> mlua::Result<()> {
    if self.child.is_none() {
      let instance = SubprocessInstance::try_new(self._command.clone(), self._args.clone()).await?;
      self.child = Some(instance);
    }
    Ok(())
  }

  async fn write_in(&mut self, data: String) -> mlua::Result<()> {
    if let Some(child) = &mut self.child {
      child
        .write_in(data)
        .await
        .map_err(|e| mlua::Error::RuntimeError(format!("Failed to write to stdin: {e}")))?;
      Ok(())
    } else {
      error!("Failed to write to stdin: cannot borrow child");
      Err(mlua::Error::RuntimeError("Process not spawned".to_string()))
    }
  }

  async fn read_out(&mut self) -> mlua::Result<Option<String>> {
    if let Some(child) = &mut self.child {
      let r = child
        .read_out()
        .await
        .map_err(|e| mlua::Error::RuntimeError(format!("Failed to read from stdout: {e}")))?;
      Ok(r)
    } else {
      error!("Failed to read from stdout: cannot borrow child");
      return Err(mlua::Error::RuntimeError("Process not spawned".to_string()));
    }
  }

  async fn read_err(&mut self) -> mlua::Result<Option<String>> {
    if let Some(child) = &mut self.child {
      let r = child
        .read_err()
        .await
        .map_err(|e| mlua::Error::RuntimeError(format!("Failed to read from stderr: {e}")))?;
      Ok(r)
    } else {
      error!("Failed to read from stderr: cannot borrow child");
      return Err(mlua::Error::RuntimeError("Process not spawned".to_string()));
    }
  }

  async fn close_stdin(&mut self) -> mlua::Result<()> {
    if let Some(child) = &mut self.child {
      child
        .close_stdin()
        .await
        .map_err(|e| mlua::Error::RuntimeError(format!("Failed to close stdin: {e}")))?;
      Ok(())
    } else {
      error!("Failed to close stdin: cannot borrow child");
      return Err(mlua::Error::RuntimeError("Process not spawned".to_string()));
    }
  }

  async fn read_out_to_end(&mut self) -> mlua::Result<Option<String>> {
    if let Some(child) = &mut self.child {
      let r= child
        .read_out_to_end()
        .await
        .map_err(|e| mlua::Error::RuntimeError(format!("Failed to read from stdout: {e}")))?;
      Ok(r)
    } else {
      error!("Failed to read from stdout: cannot borrow child");
      return Err(mlua::Error::RuntimeError("Process not spawned".to_string()));
    }
  }

  async fn read_err_to_end(&mut self) -> mlua::Result<Option<String>> {
    if let Some(child) = &mut self.child {
      let r= child
        .read_err_to_end()
        .await
        .map_err(|e| mlua::Error::RuntimeError(format!("Failed to read from stderr: {e}")))?;
      Ok(r)
    } else {
      error!("Failed to read from stderr: cannot borrow child");
      return Err(mlua::Error::RuntimeError("Process not spawned".to_string()));
    }
  }

  async fn wait(&mut self) -> mlua::Result<i32> {
    if let Some(child) = &mut self.child {
      let code = child
        .wait()
        .await
        .map_err(|e| mlua::Error::RuntimeError(format!("Failed to wait for process: {e}")))?;
      Ok(code)
    } else {
      error!("Failed to wait for process: cannot borrow child");
      Err(mlua::Error::RuntimeError("Process not spawned".to_string()))
    }
  }

  async fn terminate(&mut self) -> mlua::Result<()> {
    if let Some(child) = &mut self.child {
      child
        .terminate()
        .await
        .map_err(|e| mlua::Error::RuntimeError(format!("Failed to terminate process: {e}")))?;
      Ok(())
    } else {
      error!("Failed to terminate process: cannot borrow child");
      Err(mlua::Error::RuntimeError("Process not spawned".to_string()))
    }
  }

  async fn kill(&mut self) -> mlua::Result<()> {
    if let Some(child) = &mut self.child {
      child.kill().await.map_err(|e| mlua::Error::RuntimeError(format!("Failed to kill process: {e}")))?;
      Ok(())
    } else {
      error!("Failed to kill process: cannot borrow child");
      Err(mlua::Error::RuntimeError("Process not spawned".to_string()))
    }
  }
}

impl UserData for Subprocess {
  fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
    fields.add_field_method_get("cmd", |_, this: &Subprocess| Ok(this._command.clone()));
    fields.add_field_method_get("args", |_, this: &Subprocess| Ok(this._args.clone()));
  }

  fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
    debug!("Registering Subprocess methods");
    methods.add_async_method_mut(
      "spawn",
      |_, mut this: mlua::UserDataRefMut<Subprocess>, _: ()| async move { this.spawn().await },
    );
    methods.add_async_method_mut(
      "write_in",
      |_, mut this: mlua::UserDataRefMut<Subprocess>, data: String| async move { this.write_in(data).await },
    );
    methods.add_async_method_mut(
      "read_out",
      |_, mut this: mlua::UserDataRefMut<Subprocess>, _: ()| async move { this.read_out().await },
    );
    methods.add_async_method_mut(
      "read_err",
      |_, mut this: mlua::UserDataRefMut<Subprocess>, _: ()| async move { this.read_err().await },
    );
    methods.add_async_method_mut(
      "close_stdin",
      |_, mut this: mlua::UserDataRefMut<Subprocess>, _: ()| async move { this.close_stdin().await },
    );
    methods.add_async_method_mut(
      "read_out_to_end",
      |_, mut this: mlua::UserDataRefMut<Subprocess>, _: ()| async move { this.read_out_to_end().await },
    );
    methods.add_async_method_mut(
      "read_err_to_end",
      |_, mut this: mlua::UserDataRefMut<Subprocess>, _: ()| async move { this.read_err_to_end().await },
    );
    methods.add_async_method_mut(
      "wait",
      |_, mut this: mlua::UserDataRefMut<Subprocess>, _: ()| async move { this.wait().await },
    );
    methods.add_async_method_mut(
      "terminate",
      |_, mut this: mlua::UserDataRefMut<Subprocess>, _: ()| async move { this.terminate().await },
    );
    methods.add_async_method_mut(
      "kill",
      |_, mut this: mlua::UserDataRefMut<Subprocess>, _: ()| async move { this.kill().await },
    );
  }
}

fn create_subprocess(lua: &mlua::Lua, (command, args): (String, Vec<String>)) -> mlua::Result<AnyUserData> {
  let subprocess = Subprocess::new(command, args);
  let subprocess = lua.create_userdata(subprocess)?;
  Ok(subprocess)
}

async fn run_with_output(_: mlua::Lua, (command, args): (String, Vec<String>)) -> mlua::Result<(String, String, i32)> {
  let mut command = Command::new(command);
  command.args(args);
  let output = command
    .output()
    .await
    .map_err(|e| mlua::Error::RuntimeError(format!("Failed to run command: {e}")))?;
  let status = output.status.code().unwrap_or(-1);
  let stdout = String::from_utf8_lossy(&output.stdout).to_string();
  let stderr = String::from_utf8_lossy(&output.stderr).to_string();
  Ok((stdout, stderr, status))
}

pub(super) fn register(lua: &mlua::Lua, f_table: &mlua::Table) -> mlua::Result<()> {
  f_table.set("create_subprocess", lua.create_function(create_subprocess)?)?;
  f_table.set("run_with_output", lua.create_async_function(run_with_output)?)?;
  Ok(())
}
