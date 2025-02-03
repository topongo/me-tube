import 'dart:async';

import 'package:http/http.dart' as http;

class StreamMultipartRequest extends http.MultipartRequest {
  /// Creates a new [MultipartRequest].
  StreamMultipartRequest(
    super.method,
    super.url, {
    this.onProgress,
  });

  final void Function(int bytes, int totalBytes)? onProgress;

  /// Freezes all mutable fields and returns a single-subscription [ByteStream]
  /// that will emit the request body.
  @override
  http.ByteStream finalize() {
    final byteStream = super.finalize();
    if (onProgress == null) return byteStream;

    final total = contentLength;
    int bytes = 0;

    final t = StreamTransformer.fromHandlers(
      handleData: (List<int> data, EventSink<List<int>> sink) {
        bytes += data.length;
        onProgress!(bytes, total);
        if(total >= bytes) {
           sink.add(data);
        }
      },
    );
    final stream = byteStream.transform(t);
    return http.ByteStream(stream);
  }
}
