FROM golang:1.22-bookworm

WORKDIR /app

# Copy module files
COPY server/go.mod server/go.sum ./

# Copy shared crates/modules


# Download dependencies
RUN go mod download

# Copy server source
COPY server .

# Build
RUN go build -o /server_app .

EXPOSE 8443
EXPOSE 8444

CMD ["/server_app"]