FROM ghcr.io/cirruslabs/flutter AS frontend

WORKDIR /flutter

COPY frontend/pubspec.yaml .
RUN mkdir lib assets web && \
    echo 'main() {}' > lib/main.dart && \
    touch web/index.html && \
    sed -i -E 's/^\s+- assets.*//' pubspec.yaml
RUN flutter pub get && flutter build web
COPY frontend/pubspec.yaml .
COPY frontend/lib lib
COPY frontend/web web
COPY frontend/assets assets
RUN flutter build web

FROM nginx:alpine

COPY nginx /etc/nginx/conf.d
COPY static /static
COPY --from=frontend /flutter/build/web /flutter
