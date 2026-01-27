# Troubleshooting Guide - Hermes P2P Testing

This guide helps you diagnose and fix common issues with the P2P testing environment.

## Quick Diagnostic Commands

```bash
just validate-prereqs           # Check prerequisites
just test-pubsub-propagation    # Comprehensive health check
just status                     # Show node endpoints
just troubleshoot               # Generate full diagnostic report
```

---

## Common Issues and Solutions

### 1. Test Fails: "No nodes received the message"

**Symptoms:**

* `just test-pubsub-propagation` fails
* No nodes show RECEIVED status
* Mesh appears broken

**Diagnosis:**

```bash
just test-pubsub-propagation
just logs | grep -E '(gossipsub|bootstrap|RECEIVED)'
```

**Common Causes & Solutions:**

#### A. Mesh Not Formed

```bash
# Check if peers are connected
just check-connectivity

# If no connections, wait longer (mesh takes 15-30s to form)
sleep 30 && just test-pubsub-propagation

# If still failing, restart nodes
just restart
```

#### B. Bootstrap Issues (Peer ID Mismatch)

```bash
# Check for "wrong peer id" errors
just logs | grep -i "wrong peer id"

# Fix: Reinitialize bootstrap configuration
just init-bootstrap
```

#### C. Nodes Not Subscribed to Topic

```bash
# Check logs for PubSub subscription messages
just logs | grep -i "subscribed"

# Fix: Restart to reinitialize PubSub
just restart
```

---

### 2. Nodes Won't Start

**Symptoms:**

* `just start` fails
* Docker containers exit immediately
* Port conflicts

**Diagnosis:**

```bash
just validate-prereqs
docker compose ps
docker compose logs --tail=50
```

**Common Causes & Solutions:**

#### A. Missing Build Artifacts

```bash
# Error: "Binary not found"
just build-all
just build-images
just start
```

#### B. Port Already in Use

```bash
# Check what's using ports
lsof -i :7878
netstat -tuln | grep -E ':(7878|7880|7882|7884|7886|7888)'

# Fix: Stop existing instances
just stop
# Or kill other processes using the ports
```

#### C. Docker Not Running

```bash
# Error: "Cannot connect to the Docker daemon"
sudo systemctl start docker
# Or start Docker Desktop
```

#### D. Old Containers Still Running

```bash
# Check for stale containers
docker ps -a | grep hermes

# Remove them
just clean
just start
```

---

### 3. Partial Propagation (Some Nodes Don't Receive)

**Symptoms:**

* 1-3 nodes receive messages, but not all
* Intermittent failures

**Diagnosis:**

```bash
just test-pubsub-propagation
just logs | grep "peer connected" | wc -l  # Should be 30 for full mesh
```

**Common Causes & Solutions:**

#### A. Mesh Still Forming

```bash
# Wait longer for mesh to stabilize
sleep 30

# Check peer connection count
just check-connectivity

# Should see "30 peer connections" for full mesh
```

#### B. Network Latency

```bash
# Wait longer before checking reception
sleep 10 && just test-pubsub-propagation

# The test waits 3s for propagation by default
# If you need more time consistently, you can increase MAX_WAIT
# in the _test-pubsub-execute recipe (currently 5s total)
```

#### C. Gossipsub Configuration

```bash
# Check if Gossipsub is active
just logs | grep -i gossipsub | wc -l

# Should see many Gossipsub messages
# If none, nodes may not be initializing PubSub correctly
just restart
```

---

### 4. Docker Build Failures

**Symptoms:**

* `just build-images` fails
* COPY errors in Dockerfile

**Common Causes & Solutions:**

#### A. Artifacts Not Built

```bash
# Ensure binaries exist before building images
ls -lh ../hermes/target/release/hermes
ls -lh ../hermes/apps/athena/athena.happ

# Build them first
just build-all
just build-images
```

#### B. Docker Disk Space

```bash
# Check disk space
df -h

# Clean up Docker resources
docker system prune -a
docker volume prune

# Then rebuild
just build-images
```

---

### 5. Bootstrap Initialization Fails

**Symptoms:**

* `just init-bootstrap` fails to discover peer IDs
* "Failed to discover peer IDs from logs"

**Diagnosis:**

```bash
docker compose logs hermes-node1 hermes-node2 hermes-node3 | grep "Peer ID"
```

**Common Causes & Solutions:**

#### A. Nodes Not Starting Fast Enough

```bash
# Wait longer for nodes to fully start, then retry
sleep 30 && just init-bootstrap

# The init-bootstrap recipe waits 15s by default
# If nodes consistently need more time, wait before running it
```

#### B. Volumes Corrupted

```bash
# Nuclear option: Complete clean reset
just clean
docker volume prune -f
just build-all
just build-images
just init-bootstrap
```

---

### 6. Performance Issues / Slow Propagation

**Symptoms:**

* Messages take >10s to propagate
* High CPU usage
* Slow logs

**Diagnosis:**

```bash
docker stats
just logs | tail -100 | grep -E 'slow|timeout|latency'
```

**Solutions:**

#### A. Too Much Logging

```bash
# Reduce log verbosity in docker-compose.yml
# Change RUST_LOG from debug to info
# Then restart: just restart
```

#### B. System Resources

```bash
# Check system load
top
htop

# Stop other Docker containers
docker ps
docker stop <other-containers>

# Increase Docker resource limits in Docker Desktop settings
```

---

## Diagnostic Checklist

When things go wrong, run through this checklist:

* [ ] **Prerequisites OK?** → `just validate-prereqs`
* [ ] **Docker running?** → `docker info`
* [ ] **Nodes running?** → `docker compose ps`
* [ ] **All 6 nodes up?** → Should show 6 containers "Up"
* [ ] **Ports available?** → `lsof -i :7878` (should be empty or show our containers)
* [ ] **Peer connections?** → `just check-connectivity` (should see 30 connections)
* [ ] **Gossipsub active?** → `just logs | grep -i gossipsub` (should see many entries)
* [ ] **Bootstrap OK?** → `just logs | grep "All bootstrap peers connected"`
* [ ] **Disk space OK?** → `df -h` (need >5GB)
* [ ] **Artifacts exist?** → `ls -lh ../hermes/target/release/hermes`

---

## Nuclear Options (When All Else Fails)

### Complete Reset

```bash
# Nuclear option: Remove EVERYTHING (p2p-testing + all unused Docker resources)
docker compose down -v
docker volume prune -f   # Removes ALL unused volumes system-wide
docker network prune -f  # Removes ALL unused networks system-wide

# Rebuild and start fresh (handles build-all → build-images → start → test)
just quickstart
```

**Less aggressive alternative:**

```bash
just clean       # Only removes p2p-testing volumes
just quickstart  # Rebuilds and starts
```

### Verify Docker Setup

```bash
# Test basic Docker functionality
docker run hello-world

# Check Docker Compose version
docker compose version  # Should be v2.x

# Restart Docker daemon
sudo systemctl restart docker
# Or restart Docker Desktop
```

---

## Getting More Help

### Collect Diagnostics

```bash
# Generate comprehensive report
just troubleshoot

# This creates: p2p-troubleshoot-TIMESTAMP.txt
# Share this file when asking for help
```

### Useful Log Commands

```bash
# View all logs
just logs

# Filter for errors
just logs | grep -i error

# Check specific node
just logs-node 1

# Follow PubSub messages
just logs | grep -i "RECEIVED PubSub message"

# Check bootstrap status
just logs | grep -i bootstrap

# View peer connections
just logs | grep "peer connected"
```

### Debug Mode

```bash
# Enable more verbose logging in docker-compose.yml
# Change RUST_LOG to:
RUST_LOG=hermes::ipfs=trace,rust_ipfs=trace,libp2p=trace

# Then restart
just restart
```

---

## Understanding Error Messages

### "wrong peer id"

* **Meaning:** Bootstrap configuration has stale peer IDs
* **Fix:** `just init-bootstrap`

### "Failed to post message"

* **Meaning:** HTTP API not responding or Athena app not loaded
* **Fix:** Check node 1 logs, ensure app loaded

### "Mesh not formed"

* **Meaning:** Nodes haven't connected to each other
* **Fix:** Wait longer, check bootstrap, verify network

### "Port already in use"

* **Meaning:** Another process is using required ports
* **Fix:** `just stop` or kill conflicting process

### "Permission denied" (Docker)

* **Meaning:** User not in docker group
* **Fix:** `sudo usermod -aG docker $USER` then logout/login

---

## Prevention Tips

1. **Always validate before starting:**

   ```bash
   just validate-prereqs
   ```

1. **Use quickstart for first-time setup:**

   ```bash
   just quickstart
   ```

1. **Don't run `just clean` unless you want to delete everything**

   * `just stop` preserves data
   * `just clean` deletes volumes and peer identities

1. **Wait for mesh to form before testing:**

   ```bash
   just start
   sleep 30  # Give it time
   just test-pubsub-propagation
   ```

1. **Check health regularly:**

   ```bash
   just test-pubsub-propagation
   just status
   ```

---

## Still Stuck?

If none of these solutions work:

1. Run `just troubleshoot` and save the report
2. Check the justfile comments for detailed documentation
3. Review Docker logs: `docker compose logs > full-logs.txt`
4. Report the issue with:
   * Output of `just troubleshoot`
   * Output of `just test-pubsub-propagation`
   * Steps to reproduce the problem
   * Your system info (OS, Docker version)
