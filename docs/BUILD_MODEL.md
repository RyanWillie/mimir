# Model Download and Build Process

This document describes the automated model download and build process for the Mimir AI Memory Vault.

## Overview

The build system automatically downloads the BGE-small-en model from Hugging Face during the build process. This ensures that the model is available for vector embedding generation without manual intervention.

## File Structure

After a successful build, the following structure will be created:

```
assets/
└── bge-small-en-int8/
    ├── model-int8.onnx     (~23 MB)
    ├── tokenizer.json      (~250 KB)
    └── vocab.txt           (~200 KB)
```

## Build Process

### Automatic Download

When you run `cargo build` in the `crates/mimir` directory, the build script (`build.rs`) will:

1. Check if the model files already exist
2. If not, download them from Hugging Face:
   - `https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/onnx/model.onnx`
   - `https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/tokenizer.json`
   - `https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/vocab.txt`
3. Quantize the model to int8 format (if `optimum-cli` is available)
4. Verify SHA-256 checksums
5. Complete the build

### Cross-Platform Support

The download process works on:
- **Unix-like systems**: Uses `curl` for downloads
- **Windows**: Tries `curl` first, falls back to PowerShell's `Invoke-WebRequest`

### Opt-out Environment Variable

To skip the model download (useful for CI or offline builds), set:

```bash
export MIMIR_SKIP_MODEL_DL=1
cargo build
```

This will emit a warning and continue with the build without downloading the model.

## Checksum Verification

The build script verifies SHA-256 checksums of downloaded files to ensure integrity. If you need to update the checksums:

1. Download the files manually or let the build script download them
2. Run the checksum calculation script:

```bash
cd scripts
cargo run --bin calculate_checksums
```

3. Copy the output constants to `crates/mimir/build.rs`

## Dependencies

### Build Dependencies

The build script requires:
- `curl` (or PowerShell on Windows)
- `optimum-cli` (optional, for int8 quantization)

### Rust Dependencies

The build script uses:
- `sha2` - For SHA-256 checksum calculation
- `hex` - For hex encoding of checksums

## Troubleshooting

### Download Failures

If downloads fail:
1. Check your internet connection
2. Verify that the Hugging Face URLs are accessible
3. On Windows, ensure `curl` is available or PowerShell can access the internet

### Quantization Failures

If quantization fails:
1. Install `optimum-cli`: `pip install optimum[onnxruntime]`
2. The build will continue without quantization if `optimum-cli` is not available

### Checksum Mismatches

If checksum verification fails:
1. The files may have been corrupted during download
2. Delete the `assets/` directory and rebuild
3. If the issue persists, update the checksums using the calculation script

## CI/CD Integration

For CI/CD environments:

1. **Skip downloads**: Set `MIMIR_SKIP_MODEL_DL=1`
2. **Pre-download models**: Download models in a separate step and cache them
3. **Use pre-built images**: Include models in Docker images

Example GitHub Actions workflow:

```yaml
- name: Build Mimir
  env:
    MIMIR_SKIP_MODEL_DL: 1
  run: cargo build --release
```

## Security Considerations

- Model files are downloaded over HTTPS from Hugging Face
- SHA-256 checksums verify file integrity
- The `assets/` directory is gitignored to avoid committing large model files
- Models are processed locally and not uploaded anywhere

## Performance Notes

- Model download adds ~23MB to the build process
- Quantization reduces model size and improves inference speed
- The build script only downloads if files don't exist
- Subsequent builds skip the download step 