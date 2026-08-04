FROM nginx:alpine
COPY nginx.conf.template /etc/nginx/conf.d/default.conf.template
CMD ["/bin/sh", "-c", "envsubst '${TPHOME_UPSTREAM}' < /etc/nginx/conf.d/default.conf.template > /etc/nginx/conf.d/default.conf && nginx -g 'daemon off;'"]
