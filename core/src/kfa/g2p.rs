//! Grapheme-to-phoneme via Phonetisaurus FST composition.
//! G2P FST model is embedded at compile time from `data/g2p.fst`.

use anyhow::{anyhow, Result};
use rustfst::algorithms::{compose, shortest_path};
use rustfst::prelude::*;

static G2P_MODEL_BYTES: &[u8] = include_bytes!("data/g2p.fst");

/// A loaded G2P FST model. Thread-safe — clone is cheap (Arc-backed internally).
pub struct G2pModel {
    fst: VectorFst<TropicalWeight>,
}

impl G2pModel {
    /// Load the embedded G2P model.
    pub fn new() -> Result<Self> {
        let fst = VectorFst::<TropicalWeight>::load(G2P_MODEL_BYTES)?;
        Ok(Self { fst })
    }

    /// Convert a Khmer word into its phonemic representation.
    /// Returns a list of phoneme strings.
    pub fn phoneticize(&self, text: &str) -> Result<Vec<String>> {
        let isyms = self
            .fst
            .input_symbols()
            .ok_or_else(|| anyhow!("input symbol table missing from G2P FST"))?;

        let mut inputs: Vec<Label> = Vec::with_capacity(text.chars().count());
        for ch in text.chars() {
            if let Some(label) = isyms.get_label(ch.to_string()) {
                inputs.push(label);
            }
        }

        let mut input_fst: VectorFst<TropicalWeight> = VectorFst::new();
        let mut state = input_fst.add_state();
        input_fst.set_start(state)?;
        for sym in inputs {
            let next_state = input_fst.add_state();
            input_fst.add_tr(state, Tr::new(sym, sym, TropicalWeight::one(), next_state))?;
            state = next_state;
        }
        input_fst.set_final(state, TropicalWeight::one())?;

        let composed_fst: VectorFst<TropicalWeight> =
            compose::compose::<_, _, VectorFst<TropicalWeight>, _, _, _>(input_fst, &self.fst)?;

        let shortest_fst: VectorFst<TropicalWeight> = shortest_path(&composed_fst)?;
        let osyms = shortest_fst
            .output_symbols()
            .ok_or_else(|| anyhow!("output symbol table missing from G2P FST"))?;

        let mut phonemes = Vec::new();
        for path in shortest_fst.paths_iter() {
            for label in path.olabels {
                if label == 2 {
                    continue; // skip epsilon
                }
                if let Some(symbol) = osyms.get_symbol(label) {
                    phonemes.push(symbol.replace('|', ""));
                }
            }
        }
        Ok(phonemes)
    }
}
