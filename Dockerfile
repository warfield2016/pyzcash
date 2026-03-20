FROM rust:1.85-slim AS builder

RUN apt-get update && apt-get install -y python3 python3-pip python3-venv && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml pyproject.toml ./
COPY src/ src/
COPY python/ python/

RUN python3 -m venv /app/.venv && \
    /app/.venv/bin/pip install --no-cache-dir maturin && \
    /app/.venv/bin/maturin build --release && \
    /app/.venv/bin/pip install --no-cache-dir target/wheels/*.whl

COPY demo/ demo/
COPY tests/ tests/
COPY examples/ examples/
COPY LICENSE README.md ./

ENV PATH="/app/.venv/bin:$PATH"

EXPOSE 8080
CMD ["python3", "demo/server.py"]
