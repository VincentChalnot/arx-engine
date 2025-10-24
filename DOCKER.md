# Docker Compose Setup Testing Guide

## Prerequisites
- Docker and Docker Compose installed
- Port 8080 and 3000 available on your host

## Testing Steps

### 1. Basic Setup Test
```bash
# Start the services
docker compose up --build

# In another terminal, verify services are running
docker compose ps
```

Expected output:
- Both `nginx` and `server` services should be running
- nginx should be bound to port 8080
- server should be bound to port 3000

### 2. Test Server URL Configuration

The server URL is configured via the `SERVER_URL` environment variable. By default, it's set to `http://server:3000` (internal Docker network).

#### Test with default configuration:
```bash
docker compose up
```

Then open a browser and navigate to `http://localhost:8080`. 

#### Test with custom SERVER_URL:
```bash
# Create a .env file
echo "SERVER_URL=http://localhost:3000" > .env

# Start the services
docker compose up
```

### 3. Verify the Configuration

1. Open `http://localhost:8080` in your browser
2. Open the browser's developer console (F12)
3. Check the Network tab to see if requests are being made to the correct server URL
4. View page source and verify the meta tag is injected:
   ```html
   <head><meta name="server-url" content="http://server:3000">
   ```
5. Check that the JavaScript console shows no errors related to server connectivity

### 4. Test Gameplay

1. Click on a piece to see possible moves (highlighted in green)
2. Click on a highlighted square to move the piece
3. If you move a stacked piece, you should see a modal asking whether to move the full stack or just the top piece

### 5. Test Server Endpoints Directly

You can test the server directly:

```bash
# Get a new game state
curl -X GET http://localhost:3000/new

# The server should return binary data (board state)
```

## Troubleshooting

### Services won't start
- Check if ports 8080 or 3000 are already in use
- Review logs with `docker compose logs`

### Server URL not being injected
- Check nginx logs: `docker compose logs nginx`
- Verify the nginx.conf template is being processed correctly
- Check if sub_filter is working by viewing page source

### Server connection errors in browser
- Verify the server container is running: `docker compose ps`
- Check server logs: `docker compose logs server`
- Verify the SERVER_URL in the meta tag matches your network setup
- For local testing, if accessing from outside Docker, you might need `SERVER_URL=http://localhost:3000`

## Clean Up

```bash
# Stop services
docker compose down

# Remove images
docker compose down --rmi all

# Remove volumes (if any)
docker compose down -v
```
