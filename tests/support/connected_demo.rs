//! A temporary Demo project whose CLI acts as the default agent.
#![allow(dead_code)]

use std::path::Path;

use serde_json::Value;

use crate::common::{agpj_with_connection, connect_default_agent, demo_project};

pub struct ConnectedDemo {
    home: tempfile::TempDir,
    secret: String,
}

impl ConnectedDemo {
    pub fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        demo_project(home.path());
        let secret = connect_default_agent(home.path(), "Demo");
        Self { home, secret }
    }

    pub fn path(&self) -> &Path {
        self.home.path()
    }

    pub fn raw(&self, args: &[&str]) -> (i32, String, String) {
        agpj_with_connection(self.path(), &self.secret, args)
    }

    pub fn json_ok(&self, args: &[&str]) -> Value {
        let (code, out, err) = self.raw(args);
        assert_eq!(code, 0, "{args:?} failed: {err}");
        serde_json::from_str(&out).unwrap()
    }

    pub fn json_error(&self, args: &[&str]) -> (i32, String) {
        let (code, _, err) = self.raw(&[&["--json"], args].concat());
        assert_ne!(code, 0, "{args:?} unexpectedly succeeded");
        let error: Value = serde_json::from_str(&err).unwrap();
        (code, error["error"]["code"].as_str().unwrap().to_owned())
    }
}
