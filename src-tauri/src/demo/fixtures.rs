use crate::error::{Error, Result};

/// Named source fragments. A file version is a recipe listing fragments in order, so a
/// moved function is literally the same fragment placed somewhere else.
pub struct Parts(&'static str);

pub static WIRE: Parts = Parts(include_str!("fixtures/wire.parts"));
pub static AUTH: Parts = Parts(include_str!("fixtures/auth.parts"));
pub static WEB: Parts = Parts(include_str!("fixtures/web.parts"));
pub static REPORT: Parts = Parts(include_str!("fixtures/report.parts"));
pub static RELAY: Parts = Parts(include_str!("fixtures/relay.parts"));
pub static DOCS: Parts = Parts(include_str!("fixtures/docs.parts"));

const MARKER: &str = "@@ ";

impl Parts {
    pub fn part(&self, name: &str) -> Result<&'static str> {
        let mut sections = self.0.split(MARKER);
        sections
            .find_map(|section| {
                let (header, body) = section.split_once('\n')?;
                (header.trim() == name).then(|| body.trim_matches('\n'))
            })
            .ok_or_else(|| Error::invalid(format!("demo fixture part `{name}` is missing")))
    }

    /// Joins the named parts with `blank_lines` empty lines between them.
    pub fn file(&self, names: &[&str], blank_lines: usize) -> Result<String> {
        let parts = names.iter().map(|name| self.part(name)).collect::<Result<Vec<_>>>()?;
        Ok(join(&parts, blank_lines))
    }

    /// Wraps the named members in a scope, such as an `impl` block or a class body.
    pub fn scope(&self, open: &str, members: &[&str], close: &str) -> Result<String> {
        let members = self.file(members, 1)?;
        Ok(format!("{open}\n{}{close}", members))
    }
}

pub fn join(parts: &[&str], blank_lines: usize) -> String {
    let mut text = parts.join(&"\n".repeat(blank_lines + 1));
    text.push('\n');
    text
}
