FROM rabbitmq:3-management
MAINTAINER Julius de Bruijn

RUN rabbitmq-plugins --offline enable rabbitmq_consistent_hash_exchange
RUN rabbitmq-plugins --offline enable rabbitmq_management
ADD rabbitmq.config /etc/rabbitmq/
ADD definitions.json /etc/rabbitmq/
