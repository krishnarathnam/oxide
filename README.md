## Oxide – Simple TF‑IDF Search over XML Docs

`oxide` is a small Rust project that builds a TF‑IDF index over a folder of XML documents and exposes a tiny HTTP API (plus static HTML/JS) for searching them.

It:
- **Parses XML files** recursively from a directory.
- **Tokenizes text** into words and numbers.
- **Builds TF‑IDF scores** for each document.
- **Serves an HTTP endpoint** so you can `POST` a query and get ranked results.

### What is TF‑IDF?

TF‑IDF (Term Frequency–Inverse Document Frequency) is a classic way to score how relevant a document is to a text query.

- **Term Frequency (TF)**: how often a term appears in a document, normalized by the document’s length. For a term \(t\) in document \(d\):

  \[
  \mathrm{tf}(t, d) = \frac{\text{count of } t \text{ in } d}{\text{total number of tokens in } d}
  \]

- **Document Frequency (DF)**: in how many documents the term appears at least once.

- **Inverse Document Frequency (IDF)**: gives higher weight to terms that appear in fewer documents:

  \[
  \mathrm{idf}(t) = \log_{10}\left(\frac{N}{\mathrm{df}(t)}\right)
  \]

  where \(N\) is the number of documents.

For each query token, `oxide` multiplies TF and IDF and sums over all tokens to get a **relevance score** for every document:

\[
\mathrm{score}(d, q) = \sum_{t \in q} \mathrm{tf}(t, d) \cdot \mathrm{idf}(t)
\]

Documents are then sorted by this score in descending order.

---

## Getting Started

### Prerequisites

- **Rust** (stable, with `cargo`), e.g. installed via `rustup`.
- A folder of **XML documents** you want to index.

### Clone the Repository

From GitHub (or any other remote, such as a mirror of `docs.gl`):

```bash
git clone https://github.com/<your-user-or-org>/oxide.git
cd oxide
```

If you want to index an existing documentation repo (for example, cloning a `docs.gl`-style GitHub repo of OpenGL docs):

```bash
# Inside the oxide repo
git clone https://github.com/<org>/<docs-repo>.git docs
```

Now you have:
- `oxide/` – this project
- `oxide/docs/` – the XML (or XML-like) documentation you want to index

---

## Building the Project

You can run in debug mode:

```bash
cargo run index ./docs
```

For better performance (recommended for serving):

```bash
cargo build --release
```

The binary will be at:

- `target/release/oxide`

---

## Commands

The binary supports three subcommands:

- **`index <folder>`**: recursively walks `<folder>`, reads XML files, and writes `index.json`.
- **`search <index.json>`**: simple check of how many files are in the index.
- **`serve <index.json> [address]`**: starts an HTTP server and web UI backed by the TF‑IDF index.

### 1. Build an Index

Using the previously cloned documentation folder (e.g. `./docs`):

```bash
# Debug build
cargo run index ./docs

# Or using the release binary
target/release/oxide index ./docs
```

This will produce an `index.json` file in the project root containing:
- Per‑file term frequencies.
- Global document frequencies.

### 2. Check the Index

```bash
cargo run search index.json
```

or

```bash
target/release/oxide search index.json
```

You should see output like:

```text
Index.json contains: <N> files
```

### 3. Run the HTTP Server

```bash
cargo run serve index.json
```

By default this listens on `127.0.0.1:6969`. You can override the address:

```bash
cargo run serve index.json 0.0.0.0:8080
```

or with the release binary:

```bash
target/release/oxide serve index.json 127.0.0.1:6969
```

Once running, open the browser at:

- `http://127.0.0.1:6969/`

The server also exposes a JSON‑free, text‑only API described below.

---

## HTTP API

### POST `/api/search`

- **Method**: `POST`
- **URL**: `/api/search`
- **Body**: raw text (the search query string)
- **Response**: `text/plain`, each line showing
  - file path
  - score

Example response line:

```text
/absolute/path/to/file.xml => 0.123456
```

### Example `curl` Request

Make sure the server is running first:

```bash
cargo run -- serve index.json
```

Then, in another terminal:

```bash
curl -X POST \
  -H "Content-Type: text/plain" \
  --data 'matrix vector multiply' \
  http://127.0.0.1:6969/api/search
```

You should see up to 10 lines of results, sorted by TF‑IDF score (highest first).

Another quick example using a query related to OpenGL docs (e.g. after cloning a `docs.gl`-style repo into `./docs` and indexing it):

```bash
curl -X POST \
  -d 'glBindBuffer usage' \
  http://127.0.0.1:6969/api/search
```

---

## Example Workflow with a Cloned Docs Repository

Putting it all together with a GitHub docs repository (e.g. similar to `docs.gl`):

```bash
# 1. Clone oxide
git clone https://github.com/krishnarathnam/oxide.git
cd oxide

# 2. Clone your documentation repo inside oxide
git clone https://github.com/<org>/<docs-repo>.git docs

# 3. Build an index from the docs
cargo run --release -- index ./docs

# 4. Start the server using the generated index.json
target/release/oxide serve index.json 127.0.0.1:6969

# 5. In another terminal, run a search via curl
curl -X POST \
  -H "Content-Type: text/plain" \
  --data 'your search terms here' \
  http://127.0.0.1:6969/api/search
```

You now have a simple TF‑IDF search engine over your cloned documentation.

---

## Notes & Limitations

- Only **XML files** are indexed; non‑XML files in the directory are skipped.
- Tokenization is very simple:
  - Numbers and alphanumeric words are kept.
  - Words are converted to **uppercase** for case‑insensitive matching.
- The API currently returns **plain text**, not JSON.
- The server is intended for **local use / experiments**, not production traffic.

# oxide
