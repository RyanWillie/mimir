## Gemma 3 (1B-IT) working setup in Mimir

This documents the known-good configuration to run Gemma 3 1B Instruct with Mimir using MistralRS.

### TL;DR
- Use the SafeTensors path with VisionModelBuilder for Gemma 3.
- Do not load the GGUF right now; recent Gemma 3 GGUFs declare arch = "gemma3" which the pinned mistral.rs cannot parse and will panic with: "Unknown GGUF architecture `gemma3`".

### How Mimir chooses the model
- If a `.gguf` file exists in `~/Library/Application Support/Mimir/models`, Mimir will auto-select GGUF.
- Otherwise, it uses the SafeTensors route with the model id and VisionModelBuilder for Gemma 3.
- Relevant code:
  - `crates/mimir/src/llm_service.rs`: GGUF vs directory selection
  - `crates/mimir-llm/src/mistralrs_service.rs`: VisionModelBuilder used for Gemma 3 when not GGUF

### Set up the working SafeTensors path (macOS)
1. Create a selector directory to force non-GGUF:
```bash
mkdir -p "$HOME/Library/Application Support/Mimir/models/gemma-3-1b-it-standard"
```

2. Disable GGUF auto-selection by renaming any `.gguf` files:
```bash
mv "$HOME/Library/Application Support/Mimir/models/gemma-3-1b-it-qat-q4_0.gguf" \
   "$HOME/Library/Application Support/Mimir/models/gemma-3-1b-it-qat-q4_0.gguf.bak" 2>/dev/null || true
mv "$HOME/Library/Application Support/Mimir/models/gemma-3-1b-it-q4_0.gguf" \
   "$HOME/Library/Application Support/Mimir/models/gemma-3-1b-it-q4_0.gguf.bak" 2>/dev/null || true
```

3. Optional: explicitly point Mimir to the directory (overrides auto-detection):
```bash
export MIMIR_LLM_MODEL_PATH="$HOME/Library/Application Support/Mimir/models/gemma-3-1b-it-standard"
```

4. Start Mimir. You should see logs similar to:
- "Using directory model: …/gemma-3-1b-it-standard"
- "Loading SafeTensors model with VisionModelBuilder..."

### Troubleshooting
- Panic: `Unknown GGUF architecture "gemma3"`
  - Cause: recent Gemma 3 GGUF file; not supported by current mistral.rs.
  - Fix: follow the steps above to force the SafeTensors/vision path.

- Still picking GGUF after renaming
  - Ensure no `.gguf` remains in `~/Library/Application Support/Mimir/models`.
  - Set `MIMIR_LLM_MODEL_PATH` to the selector directory to force non-GGUF.

- Cached files in Hugging Face hub (for reference)
  - GGUF snapshot dir: `~/.cache/huggingface/hub/models--google--gemma-3-1b-it-qat-q4_0-gguf/snapshots/<rev>/`
  - SafeTensors snapshot dir: `~/.cache/huggingface/hub/models--google--gemma-3-1b-it/snapshots/<rev>/`

### Notes
- The helper that downloads a Gemma 3 GGUF (`ensure_gemma3_model`) currently uses a `tree/main` URL. Direct downloads should use `resolve/main`. Until that’s updated, prefer manual management of your GGUF or use the SafeTensors/vision path as described above.
- If you specifically need GGUF for Gemma 3, update to a mistral.rs revision that supports `arch = "gemma3"`. Otherwise, keep using SafeTensors with VisionModelBuilder.

