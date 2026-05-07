# Build stage
FROM golang:1.21-bookworm AS builder

    WORKDIR /app
    COPY go.mod ./
    # go.sum will be generated if missing
    RUN go mod download

COPY . .
RUN go build -o enrollment-server .

# Final stage
FROM golang:1.21-bookworm

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/enrollment-server .

# Enrollment port
EXPOSE 8443
# mTLS communication port
EXPOSE 8444

# No CMD needed as tests will be run ephemerally
