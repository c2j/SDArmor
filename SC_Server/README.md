# SDChat Security Scanner Server

## Overview

The SDChat Security Scanner Server is the backend component of the security vulnerability scanning system. It provides rule management, report storage, and API services for the SDChat Scanner desktop client application.

## Features

- **Rule Management**: Store, update, and serve security scanning rules
- **Report Storage**: Receive and store scan reports from client applications
- **RESTful API**: Comprehensive API for client communication
- **Authentication**: JWT-based authentication for secure access
- **Admin Dashboard**: Web interface for managing rules, users, and reports (coming soon)

## Requirements

- Python 3.8+
- Flask and dependencies (see requirements.txt)
- SQLite (default) or PostgreSQL database

## Installation

1. Clone the repository:
   ```
   git clone <repository-url>
   cd SDChat-SC/SC_Server
   ```

2. Create and activate a virtual environment:
   ```
   python -m venv venv
   source venv/bin/activate  # On Windows, use: venv\Scripts\activate
   ```

3. Install dependencies:
   ```
   pip install -r requirements.txt
   ```

4. Set up environment variables:
   ```
   cp .env.example .env
   # Edit .env to configure your environment
   ```

5. Initialize the database:
   ```
   python init_db.py
   ```

## Configuration

The server can be configured using environment variables or a .env file. Key configuration options include:

- `SECRET_KEY`: Flask secret key for session security
- `DATABASE_URI`: Database connection string
- `JWT_SECRET_KEY`: Secret for JWT token generation
- `RULES_DIR`: Directory for rule storage
- `REPORTS_DIR`: Directory for report storage

See `.env.example` for all available configuration options.

## Usage

### Starting the Server

Run the server with:

```
python server.py
```

For production deployment, use Gunicorn:

```
gunicorn -w 4 -b 0.0.0.0:5000 "app:create_app()"
```

### API Endpoints

The server provides the following API endpoints:

#### Authentication

- `POST /api/v1/auth/login`: Authenticate and receive JWT tokens
- `POST /api/v1/auth/register`: Register a new user
- `POST /api/v1/auth/refresh`: Refresh access token

#### Rules

- `GET /api/v1/rules`: Get all rule sets
- `GET /api/v1/rules/active`: Get the active rule set
- `GET /api/v1/rules/<id>`: Get a specific rule set
- `POST /api/v1/rules`: Create a new rule set (admin only)
- `PUT /api/v1/rules/<id>`: Update a rule set (admin only)
- `POST /api/v1/rules/<id>/activate`: Activate a rule set (admin only)
- `DELETE /api/v1/rules/<id>`: Delete a rule set (admin only)

#### Reports

- `GET /api/v1/reports`: Get all reports for the current user
- `GET /api/v1/reports/<id>`: Get a specific report
- `POST /api/v1/reports`: Upload a new report
- `DELETE /api/v1/reports/<id>`: Delete a report
- `GET /api/v1/reports/<id>/export`: Export a report

## Development

### Project Structure

```
SC_Server/
├── app/                  # Main application package
│   ├── api/              # API blueprints and route handlers
│   ├── models/           # Database models
│   ├── static/           # Static files
│   ├── templates/        # HTML templates
│   └── utils/            # Utility functions
├── data/                 # Data storage
│   ├── rules/            # Rule definitions
│   └── reports/          # Stored reports
├── server.py             # Server entry point
├── init_db.py            # Database initialization script
├── requirements.txt      # Python dependencies
└── README.md             # This file
```

### Running Tests

```
pytest
```

## License

Copyright © 2023 SDChat Security Scanner Team. All rights reserved.