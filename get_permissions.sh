#!/bin/bash
mount -o remount,group /sys/kernel/tracing/
chgrp -R tracing /sys/kernel/tracing/
chmod -R g+rwx /sys/kernel/tracing/
chmod -R g+wx /sys/kernel/tracing/events
chmod -R g+wx /sys/kernel/tracing/uprobe_events

sysctl kernel.perf_event_paranoid=-1 -w
#sudo groupadd msr
#sudo usermod -a -G msr $USER
#sudo setcap cap_sys_rawio=ep ryzen
#sudo chgrp -R msr /dev/cpu/*/msr
#sudo chmod g+r /dev/cpu/*/msr

