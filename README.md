# 解决家庭路有dmz主机后的安全问题

本质上是一个带有认证机制的端口转发工具


* 编译：

cargo build --release

* 打包docker



# 设置混杂模式
sudo ip link set eno2 promisc on

#创建桥接网络
docker network create -d macvlan --subnet=192.168.10.0/24 --gateway=192.168.10.1 -o parent=enp88s0 macvlan
