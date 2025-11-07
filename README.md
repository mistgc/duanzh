# Duanzh - Containerized Deployment

This project is containerized for easy deployment using Docker and Kubernetes.

## Docker Setup

### Building and Running with Docker

1. Build the Docker image:
```bash
docker build -t duanzh .
```

2. Run the container (requires an LLM service at the specified URL):
```bash
docker run -p 3000:3000 \
  -e LLM_API_KEY=your_api_key \
  -e LLM_API_URL=http://your-llm-service:11434/api/generate \
  duanzh
```

## Docker Compose Setup

For local development with Ollama:

1. Make sure Docker Compose is installed
2. Run the services:
```bash
docker-compose up -d
```

The application will be available at `http://localhost:3000`

## Kubernetes Deployment

### Prerequisites
- A running Kubernetes cluster
- `kubectl` configured to connect to your cluster

### Deploying to Kubernetes

1. Build and push the Docker image to a registry (update the image name in deployment.yaml):
```bash
docker build -t your-registry/duanzh:latest .
docker push your-registry/duanzh:latest
```

2. Update the image name in `k8s/deployment.yaml`

3. Create the secret for the API key (in production):
```bash
kubectl create secret generic duanzh-secrets \
  --from-literal=llm-api-key=your_actual_api_key_here
```

4. Apply the Kubernetes manifests:
```bash
kubectl apply -f k8s/
```

5. Check the deployment status:
```bash
kubectl get pods
kubectl get services
```

## Configuration

The application uses the following environment variables:

- `LLM_API_KEY`: API key for the LLM service (default: "dummy_key")
- `LLM_API_URL`: URL for the LLM service (default: "http://localhost:11434/api/generate")
- `RUST_LOG`: Log level (default: "info")

## Architecture

- The application listens on port 3000
- It connects to an LLM service for chapter validation and analysis
- Supports UTF-8 encoded text files, including Chinese
- Provides REST API endpoints for uploading and processing text files