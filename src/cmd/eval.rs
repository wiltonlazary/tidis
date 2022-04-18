use crate::utils::{resp_err, resp_int, resp_sstr, resp_str};
use crate::{Connection, Frame, Parse};
use crate::tikv::lua::LuaCommandCtx;
use crate::config::{is_use_txn_api};
use crate::tikv::errors::AsyncResult;
use bytes::Bytes;
use mlua::{
    Lua
};


use tracing::{debug, instrument};

#[derive(Debug)]
pub struct Eval {
    script: String,
    numkeys: i64,
    keys: Vec<String>,
    args: Vec<String>,
}

impl Eval {
    pub fn new(script: &str, numkeys: i64) -> Eval {
        Eval {
            script: script.to_owned(),
            numkeys: numkeys,
            keys: vec![],
            args: vec![],
        }
    }

    /// Get the key
    pub fn keys(&self) -> &Vec<String> {
        &self.keys
    }

    pub fn add_key(&mut self, key: String) {
        self.keys.push(key);
    }

    pub fn add_arg(&mut self, arg: String) {
        self.args.push(arg);
    } 

    pub(crate) fn parse_frames(parse: &mut Parse) -> crate::Result<Eval> {
        let script = parse.next_string()?;
        let numkeys = parse.next_int()?;
        let mut eval = Eval::new(&script, numkeys);

        for _ in 0..eval.numkeys {
            if let Ok(key) = parse.next_string() {
                eval.add_key(key);
            } else {
                break;
            }
        }

        loop {
            if let Ok(arg) = parse.next_string() {
                eval.add_arg(arg);
            } else {
                break;
            }
        }

        Ok(eval)
    }

    #[instrument(skip(self, dst))]
    pub(crate) async fn apply(self, dst: &mut Connection) -> crate::Result<()> {
        let response = match self.eval().await {
            Ok(val) => val,
            Err(e) => Frame::Error(e.to_string()),
        };

        debug!(?response);

        dst.write_frame(&response).await?;

        Ok(())
    }

    async fn eval(&self) -> AsyncResult<Frame> {
        if !is_use_txn_api() {
            return Ok(resp_err("not supported yet"));
        }

        LuaCommandCtx::new(None).do_async_eval(&self.script, &self.keys, &self.args).await
    }
}
