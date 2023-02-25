FROM alpine:3.17.2
COPY target/release/dmz_locker /
ENTRYPOINT["/dmz_locker"]