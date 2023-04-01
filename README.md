# Запуск
Для выполнения требуются установленные [CUDA 11](https://developer.nvidia.com/cuda-11-8-0-download-archive) и [cuDNN](https://developer.nvidia.com/rdp/cudnn-archive) (если использование CUDA включено в настройках).

Для запуска используется программа `launcher`.

# Сборка из исходного кода
Команды выполняются относительно папки с проектом.

1. Скачайте [Elasticsearch](https://www.elastic.co/downloads/elasticsearch) (8.7.0);
2. Скачайте [Apache Tika](https://tika.apache.org/download.html) (`tika-server-standard-2.7.0.jar`);
3. Скачайте [ONNX Runtime](https://github.com/microsoft/onnxruntime/releases) (версии `*-gpu-*` поддерживают выполнение и на CPU, и на GPU с поддержкой CUDA);
4. Установите [FFmpeg](https://ffmpeg.org/download.html);
5. Скачайте [NNServer](https://github.com/ArtSin/NNServer) и сконвертируйте модели в формат ONNX;
6. Создайте `.cargo/config.toml`, укажите в нём путь к ONNX Runtime:
```toml
[env]
ORT_RUST_STRATEGY = "system"
ORT_RUST_LIB_LOCATION = ".../onnxruntime-linux-x64-gpu-1.14.1/"
ORT_RUST_USE_CUDA = "1"
```
7. Внутри папки ONNX Runtime переместите `include/onnxruntime_c_api.h` в `include/onnxruntime/core/session/onnxruntime_c_api.h`;
8. Соберите проект из исходного кода:
   1. `cd client_ui && trunk build --release && cd ..`
   2. `cargo build --release --bin indexer`
   3. `cargo build --release --bin nn_server`
   4. `cargo build --release --bin launcher`
9. Скопируйте Elasticsearch, Apache Tika, ONNX Runtime, ONNX-модели, результаты сборки из `target/release` и файлы из `install` в какую-нибудь папку так, чтобы получилась следующая структура:
```
elasticsearch-8.7.0/
    bin/
    config/
        elasticsearch.yml
        jvm.options
        ...
    ...
nn_server/
    models/
        clip-ViT-B-32/
        clip-ViT-B-32-multilingual-v1/
        mMiniLM-L6-v2-mmarco-v2/
        paraphrase-multilingual-MiniLM-L12-v2/
    nn_server(.exe)
onnxruntime-linux-x64-gpu-1.14.1/
indexer(.exe)
launcher(.exe)
tika-config.xml
tika-server-standard-2.7.0.jar
```