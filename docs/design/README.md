# SD Armor Design Documentation

This directory contains the design documentation for SD Armor.

## Architecture Overview

SD Armor follows a client-server architecture with a desktop client and web backend:

```
+------------------+     +------------------+
|   Desktop Client |     |   Web Backend    |
|   (Rust/egui)    |<--->|  (Python/Flask)  |
+------------------+     +------------------+
         |                       |
         v                       v
+------------------+     +------------------+
|  3D Visualization|     |   PostgreSQL     |
|     (wgpu)       |     |     Database     |
+------------------+     +------------------+
```

## Design Documents

1. [System Architecture](architecture.md) - High-level system architecture
2. [Data Model](data-model.md) - Database schema and entity relationships
3. [API Design](api-design.md) - REST API design principles and patterns
4. [UI/UX Design](ui-ux.md) - User interface and experience design
5. [Security Design](security.md) - Security architecture and best practices
6. [Performance Design](performance.md) - Performance optimization strategies

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

## Design Principles

1. **API-First**: All features are accessible through well-documented RESTful APIs
2. **Security First**: Comprehensive authentication and authorization for all endpoints
3. **Structured Data Modeling**: Clear entity relationships and constraints
4. **Consistent Error Handling**: Standardized error response formats
5. **Multi-Layer Testing**: Unit, integration, and contract tests for all features
6. **Documentation Standards**: Comprehensive documentation for all APIs and user interactions
7. **Code Organization**: Clean separation of concerns with models, services, and API layers
8. **Performance & Scalability**: Optimized database queries and resource usage