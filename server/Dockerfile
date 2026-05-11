FROM golang:1.21-bookworm AS builder

WORKDIR /app

COPY server/go.mod ./server/go.mod

WORKDIR /app/server

RUN go mod download

WORKDIR /app

COPY . .

RUN mkdir -p /app/server/pkg/logger
COPY logger/server/logger.go /app/server/pkg/logger/logger.go

WORKDIR /app/server

RUN go build -o enrollment-server .

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/server/enrollment-server .

EXPOSE 8443
EXPOSE 8444

CMD ["./enrollment-server"]
