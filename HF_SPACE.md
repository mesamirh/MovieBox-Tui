# Hugging Face Space deployment

This branch adds a small Rust/Axum metadata API while leaving the existing TUI intact.

## 1. Create the Space

Create a new Hugging Face Space and choose **Docker** as the SDK.

The Space README metadata should look like this:

```yaml
---
title: MovieBox API
emoji: 🎬
colorFrom: gray
colorTo: blue
sdk: docker
app_port: 7860
---
```

## 2. Push this branch to the Space

Clone this repository/branch locally, add the Hugging Face Space as another git remote, and push `hf-api` to the Space's `main` branch.

```bash
git clone -b hf-api https://github.com/DhruvKadam5911/MovieBox.git
cd MovieBox
git remote add hf https://huggingface.co/spaces/YOUR-HF-USERNAME/YOUR-SPACE-NAME
git push hf hf-api:main
```

If Hugging Face generated its own README when the Space was created, keep its YAML front matter and copy the rest of this repository into the Space.

## 3. Optional environment variables

Set these from **Space Settings → Variables and secrets**:

- `MOVIEBOX_API_KEY`: optional secret. When set, `/api/search` and `/api/details/:subject_id` require the same value in the `x-api-key` request header.
- `ALLOWED_ORIGIN`: optional browser origin such as `https://your-site.vercel.app`. If unset or set to `*`, CORS allows all origins.
- `PORT`: normally leave unset on Hugging Face; the server defaults to `7860`.

## Endpoints

```text
GET /
GET /health
GET /api/search?q=interstellar&page=1
GET /api/details/<subject_id>
```

Example:

```js
const API = "https://YOUR-SPACE.hf.space";

const response = await fetch(
  `${API}/api/search?q=${encodeURIComponent("Interstellar")}&page=1`,
  {
    headers: {
      "x-api-key": "YOUR_API_KEY"
    }
  }
);

const result = await response.json();
console.log(result);
```

If `MOVIEBOX_API_KEY` is not configured, omit the `x-api-key` header.

## Scope

The HTTP server exposes metadata search/details only. It does not expose the TUI's playback or download paths as a public media relay.
