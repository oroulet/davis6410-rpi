set -x 
sudo cp wind.service /etc/systemd/system/wind.service
sudo mkdir -p /opt/public
sudo cp ../public/index.html /opt/index.html
sudo cp -r pu.service /etc/systemd/system/wind.service
sudo systemctl daemon-reload
sudo systemctl enable wind.service
sudo systemctl start wind.service
