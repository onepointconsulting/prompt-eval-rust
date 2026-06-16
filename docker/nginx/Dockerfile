FROM nginx:alpine

# 1. Prepare frpc
RUN mkdir -p /frpc/
COPY docker/run_frpc.sh /frpc/
ADD https://github.com/fatedier/frp/releases/download/v0.65.0/frp_0.65.0_linux_amd64.tar.gz /tmp/frp.tar.gz
RUN tar -xzf /tmp/frp.tar.gz -C /tmp/ && \
    mv /tmp/frp_0.65.0_linux_amd64/frpc /frpc/frpc && \
    chmod +x /frpc/frpc /frpc/run_frpc.sh && \
    rm -rf /tmp/frp.tar.gz /tmp/frp_0.65.0_linux_amd64

# 2. nginx config + entrypoint
COPY docker/nginx/default.conf /etc/nginx/conf.d/default.conf
COPY docker/run_nginx.sh /docker/run.sh
RUN sed -i 's/\r$//' /frpc/run_frpc.sh /docker/run.sh && chmod +x /docker/run.sh /frpc/run_frpc.sh

EXPOSE 80

CMD ["/docker/run.sh"]
