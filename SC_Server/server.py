#!/usr/bin/env python3
import os
import sys
import logging
from dotenv import load_dotenv
from app import create_app

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    handlers=[
        logging.StreamHandler(sys.stdout)
    ]
)
logger = logging.getLogger(__name__)

# Load environment variables
load_dotenv()

# Create the Flask application
app = create_app()

if __name__ == '__main__':
    import argparse
    
    parser = argparse.ArgumentParser(description='SDChat Security Scanner Server')
    parser.add_argument(
        '--host', 
        default=os.environ.get('HOST', '0.0.0.0'),
        help='Host to bind the server to'
    )
    parser.add_argument(
        '--port', 
        type=int, 
        default=int(os.environ.get('PORT', 5000)),
        help='Port to bind the server to'
    )
    parser.add_argument(
        '--debug', 
        action='store_true',
        default=os.environ.get('FLASK_DEBUG') == '1',
        help='Run in debug mode'
    )
    
    args = parser.parse_args()
    
    logger.info(f"Starting SDChat Scanner Server on {args.host}:{args.port}")
    logger.info(f"Debug mode: {'enabled' if args.debug else 'disabled'}")
    
    app.run(
        host=args.host,
        port=args.port,
        debug=args.debug
    )