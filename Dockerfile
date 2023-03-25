FROM alpine:3.12

ENV CORE_CHART_DIR="/chart"

RUN apk add --no-cache bash curl openssl

RUN curl -fsSL -o get_helm.sh https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 && \
    chmod 700 get_helm.sh && \
    ./get_helm.sh

COPY ./target/release/upgrade-job /usr/local/bin/upgrade-job

RUN mkdir /chart

COPY ./chart/* /chart/

RUN helm dependency update /chart

ENTRYPOINT ["upgrade-job"]