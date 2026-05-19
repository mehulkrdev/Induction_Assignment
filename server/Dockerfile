FROM golang:1.22-bookworm

WORKDIR /app

# Copy module files
COPY go.mod go.sum ./

# Download dependencies
RUN go mod download

# Copy server source
COPY server server

# Copy necessary third_party assets
COPY third_party third_party

# Build from the server directory
RUN go build -o /server_app ./server

EXPOSE 8443
EXPOSE 8444

CMD ["/server_app"]
