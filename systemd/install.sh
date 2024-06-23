set -x 
sudo cp wind.service /etc/systemd/system/wind.service
sudo mkdir -p /opt/public
sudo cp ../public/* /opt/public/
sudo systemctl daemon-reload
sudo systemctl enable wind.service
sudo systemctl start wind.service
