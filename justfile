binary_name := "near-monitor"
install_path := "/usr/local/bin/" + binary_name
service_name := "near-monitor"

# Build the project in release mode
build:
    cargo build --release

# Install the binary to /usr/local/bin
install: build
    -sudo systemctl stop {{service_name}}
    sudo install -m 755 target/release/app {{install_path}}
    @echo "Installed to {{install_path}}"

# Setup systemd service (requires TELEGRAM_BOT_TOKEN)
# Usage: just setup-service <your_token>
setup-service token: install
    @echo "Generating systemd service file..."
    printf "[Unit]\nDescription=NEAR Balance Monitor Bot\nAfter=network.target\n\n[Service]\nExecStart={{install_path}} bot\nRestart=always\nEnvironment=TELOXIDE_TOKEN={{token}}\nEnvironment=RUST_LOG=info\n\n[Install]\nWantedBy=multi-user.target\n" | sudo tee /etc/systemd/system/{{service_name}}.service > /dev/null
    sudo systemctl daemon-reload
    sudo systemctl enable {{service_name}}
    @echo "Service {{service_name}} installed and enabled."
    @echo "Start it with: sudo systemctl start {{service_name}}"

# Uninstall the binary and service
uninstall:
    -sudo systemctl stop {{service_name}}
    -sudo systemctl disable {{service_name}}
    sudo rm -f /etc/systemd/system/{{service_name}}.service
    sudo rm -f {{install_path}}
    sudo systemctl daemon-reload
    @echo "Uninstalled {{service_name}} and its binary."

# Stop the service
stop:
    -sudo systemctl stop {{service_name}}

# Restart the service
restart:
    sudo systemctl restart {{service_name}}

# Check the service status
status:
    sudo systemctl status {{service_name}}

# Install and restart the service
deploy: install restart

# Alias for deploy
install-restart: deploy
