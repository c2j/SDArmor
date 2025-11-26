# SD Armor Project Overview

This document provides an overview of the improvements made to the SD Armor (SD盾甲) project.

## Project Branding

### Product Name
- **Chinese Name**: SD盾甲
- **English Name**: SD Armor
- **Naming Concept**: Like the shield and armor of ancient warriors, providing solid security protection for your code

## Key Improvements

### 1. Enhanced .gitignore File
- Updated to properly ignore files for both Rust desktop client and Python backend
- Added specific ignores for SD Armor project structure
- Included ignores for development tools, IDEs, and OS-specific files

### 2. Comprehensive Documentation Structure
Created a well-organized documentation structure:
- `docs/` - Main documentation directory
  - `api/` - API documentation and specifications
  - `design/` - Design documents and architecture
  - `user/` - User guides and tutorials
- `scripts/` - Development and deployment scripts
  - `devops/` - Development environment setup and tools
  - `deployment/` - Deployment scripts
  - `utils/` - Utility scripts

### 3. Development Scripts
Created essential development scripts:
- `setup-dev-env.sh` - Sets up the complete development environment
- `run-tests.sh` - Runs all tests for both frontend and backend
- `start-dev-server.sh` - Starts the development server
- `build-desktop.sh` - Builds the desktop client

### 4. Project Structure Improvements
- Created a clear directory structure for better organization
- Separated concerns between frontend (Desktop) and backend (SC_Server)
- Added proper documentation directories

### 5. Updated README Files
- Created a comprehensive main README.md with project overview
- Added README files for each documentation section
- Provided clear instructions for installation and usage

## Technology Stack

### Frontend (Desktop Client)
- **Language**: Rust
- **Framework**: egui/eframe for GUI
- **3D Graphics**: wgpu for rendering
- **Networking**: reqwest for HTTP requests
- **Async Runtime**: tokio

### Backend (Web Service)
- **Language**: Python
- **Framework**: Flask
- **Database**: PostgreSQL
- **ORM**: SQLAlchemy
- **Authentication**: JWT tokens

## Development Workflow

1. **Setup**: Run `scripts/devops/setup-dev-env.sh` to set up the development environment
2. **Development**: Work on features in the respective directories
3. **Testing**: Run `scripts/devops/run-tests.sh` to execute all tests
4. **Running**: Use `scripts/devops/start-dev-server.sh` to start the backend server
5. **Building**: Use `scripts/devops/build-desktop.sh` to build the desktop client

## Future Enhancements

1. Add CI/CD pipeline configuration
2. Create Dockerfiles for containerized deployment
3. Add more comprehensive API documentation
4. Create user tutorials and video guides
5. Implement automated testing and deployment scripts

## Contributing

The project now has a clear structure and development tools that make it easy for new contributors to get started. All scripts are well-documented and follow consistent naming conventions.