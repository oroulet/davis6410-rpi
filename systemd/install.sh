sudo cp wind.service /etc/systemd/system/wind.service
sudo systemctl reload
sudo systemctl enable wind.service
sudo systemctl start wind.service
