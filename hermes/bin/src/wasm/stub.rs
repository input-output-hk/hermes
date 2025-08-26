//! Defines default exports in a wasm file.
#![allow(unused, clippy::all)]

use wasm_encoder::{
    reencode::{self, Reencode, ReencodeComponent},
    Component,
};
use wasmparser::{ComponentExternalKind, Parser};

pub fn stub(wasm_bytes: &[u8]) -> Result<Component, reencode::Error<anyhow::Error>> {
    let parser = Parser::default();
    let mut component = Component::new();
    Reencoder.parse_component(&mut component, parser, wasm_bytes)?;
    Ok(component)
}

struct Reencoder;

impl Reencode for Reencoder {
    type Error = anyhow::Error;
}

impl ReencodeComponent for Reencoder {
    fn parse_component_export_section(
        &mut self,
        exports: &mut wasm_encoder::ComponentExportSection,
        section: wasmparser::ComponentExportSectionReader<'_>,
    ) -> Result<(), reencode::Error<Self::Error>> {
        println!("{}", section.count());
        for export in section {
            let export = export?;
            if export.kind == ComponentExternalKind::Func {
                println!("{:?}", export);
            }
            self.parse_component_export(exports, export)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn wip_stub() {
        super::stub(include_bytes!("../../../../wasm/app-components/cardano_age.rs.wasm"));
    }
}
