#!/bin/sh
set -eu
mkdir /trusted
cp /fixtures/valid.lic /fixtures/installation.json /trusted/
head -c 32 /dev/zero > /trusted/device.key
chgrp 65534 /trusted /trusted/valid.lic /trusted/installation.json
chmod 750 /trusted
chmod 640 /trusted/valid.lic /trusted/installation.json
chmod 600 /trusted/device.key
export LD_LIBRARY_PATH=/
su -s /bin/sh nobody -c 'test -r /trusted/valid.lic && test -r /trusted/installation.json && test ! -r /trusted/device.key && exec /production-trust'
echo 'PASS: unprivileged service can read public authorization but cannot read the private device key'
