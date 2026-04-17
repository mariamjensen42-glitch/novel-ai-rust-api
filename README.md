# novel-ai-rust-api

A Rust API server using Actix Web that integrates with DeepSeek and Qwen language models.

## Features
- **Actix Web** server for fast, asynchronous API handling
- **DeepSeek** model integration
- **Qwen** model integration
- **Configuration management** via environment variables
- **Health check endpoint** for monitoring
- **Prediction endpoint** for text generation

## API Endpoints

### POST /predict
Generates text using either DeepSeek or Qwen model.

**Request Body:**
```json
{
  "model": "deepseek" or "qwen",
  "prompt": "Your prompt here",
  "max_tokens": 100, (optional)
  "temperature": 0.7 (optional)
}
```

**Response:**
```json
{
  "model": "deepseek",
  "generated_text": "Generated text here",
  "tokens_used": 42
}
```

### GET /health
Returns server health status.

**Response:**
```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

## Setup

1. **Install Rust** (if not already installed): https://www.rust-lang.org/tools/install

2. **Clone the repository**

3. **Set up environment variables**:
   - Copy `.env.example` to `.env`
   - Fill in your API keys for DeepSeek and Qwen

4. **Build and run the server**:
   ```bash
   cargo run
   ```

The server will start on `http://127.0.0.1:8080`

## Dependencies
- actix-web: Web framework
- serde: Serialization/deserialization
- reqwest: HTTP client for model integration
- tokio: Async runtime
- dotenv: Environment variable management

