FROM rust:latest

# Set environment variables
ENV DEBIAN_FRONTEND=noninteractive

# Update and install necessary dependencies
RUN apt-get update && apt-get install -y \
  libssl-dev \
  pkg-config \
  build-essential \
  curl \
  git \
  wget \
  unzip \
  chromium \
  && rm -rf /var/lib/apt/lists/*

# Install Shuttle CLI for deployment
RUN cargo install cargo-shuttle

# Set the working directory in the container
WORKDIR /app

# Copy project files into the container
COPY . .

# Command to deploy the application
CMD ["shuttle", "run"]

