current_dir = $(shell pwd)

dev_install:
	@ echo "WARNING: THIS WILL BREAK EXISTING UNIXD INSTALLS"
	@ echo "ctrl-c now if this is not what you want"
	@ read
	@ echo "LAST CHANCE"
	@ sleep 5
	ln -s -f $(current_dir)/../platform/opensuse/kubidm-unixd.service /etc/systemd/system/kubidm-unixd.service
	ln -s -f $(current_dir)/../platform/opensuse/kubidm-unixd-tasks.service /etc/systemd/system/kubidm-unixd-tasks.service
	ln -s -f $(current_dir)/../target/debug/kubidm-unix /usr/sbin/kubidm-unix
	ln -s -f $(current_dir)/../target/debug/kubidm_ssh_authorizedkeys /usr/sbin/kubidm_ssh_authorizedkeys
	ln -s -f $(current_dir)/../target/debug/kubidm_unixd_tasks /usr/sbin/kubidm_unixd_tasks
	ln -s -f $(current_dir)/../target/debug/kubidm_unixd /usr/sbin/kubidm_unixd
	ln -s -f $(current_dir)/../target/debug/libpam_kubidm.so /lib64/security/pam_kubidm.so
	ln -s -f $(current_dir)/../target/debug/libnss_kubidm.so /usr/lib64/libnss_kubidm.so.2