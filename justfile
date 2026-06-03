set dotenv-load

cvm_name := "worker"
cvm_id := env_var("PHALA_CVM_ID")

# Deploy compose + env to Phala Cloud (creates a new CVM)
cvm-upgrade-manifest:
      phala \
          deploy \
          --cvm-id {{cvm_id}} \
          --compose ./compose.yaml

cvm-attestation:
  phala cvms attestation {{cvm_id}}

cvm-logs:
  phala logs dstack-{{cvm_name}}-1 --cvm-id {{cvm_id}} -f

cvm-status:
  phala cvms get {{cvm_name}}

cvm-start:
  phala cvms start {{cvm_name}}

cvm-stop:
  phala cvms stop {{cvm_name}}


# Deploy compose + env to Phala Cloud (creates a new CVM)
cvm-init-deploy:
    phala deploy \
        --name outlayer \
        --compose compose.yaml \
        --vcpu 2 \
        --memory 2G \
        --disk-size 1G \
        --image dstack-0.5.9 \
        --kms phala

# Start / stop the CVM
start:
    phala cvms start {{cvm_name}}

stop:
    phala cvms stop {{cvm_name}}

# Stream logs
logs:
    phala cvms logs {{cvm_name}} --follow

# CVM status
status:
    phala cvms get {{cvm_name}} --json

# Build and push image to Docker Hub (linux/amd64 for Phala)
deploy-docker-image:
    docker buildx build --platform linux/amd64 \
        -t {{image}} \
        -f crates/outlayer/bin/Dockerfile \
        --push .
