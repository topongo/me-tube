import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:async/async.dart';
import 'package:dio/dio.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/foundation.dart';
import 'main.dart';
import 'package:http/http.dart' as http;
import 'package:http/io_client.dart';
import 'package:media_kit/media_kit.dart';
import 'package:shared_preferences/shared_preferences.dart';

class AuthService with ChangeNotifier {
  static String baseUrl = apiBaseUrl;
  String? _refreshToken;
  String? _accessToken;
  bool _isLoading = false;
  String? _error;
  late http.Client client;
  String? username;
  bool? isAdmin;
  bool? passwordReset;
  final Map<String, VideoToken> _videoTokens = {};

  // Getters
  bool get isAuthenticated {
    if (kIsWeb) {
      return _accessToken != null;
    } else {
      return _refreshToken != null;
    }
  }
  bool get isLoading => _isLoading;
  String? get error => _error;

  // Stream for auth state changes
  final _authStreamController = StreamController<bool>();
  Stream<bool> get authState => _authStreamController.stream;

  // Initialize from local storage (e.g., token persistence)
  AuthService() {
    // if (!kIsWeb) {
    //   final context = SecurityContext.defaultContext;
    //   context.setTrustedCertificatesBytes(certificate);
    //   client = IOClient(HttpClient(context: context));
    // } else {
      client = http.Client();
    // }
    _loadToken();
  }

  // Load token from SharedPreferences
  Future<void> _loadToken() async {
    final prefs = await SharedPreferences.getInstance();
    _refreshToken = prefs.getString('refresh_token');
    print("got token: $_refreshToken");
    if (kIsWeb) {
      _refreshToken = "ONEhJN/3OVRiKsKNaXDaa6U1KWK8CIzga/QTVh/K5e0=";
    }
    if (_refreshToken != null) {
      print("Refresh token is present: refreshing access...");
      await _refreshAccessToken();
    } else {
      print("Refresh token missing: redirect to login");
    }
    // print("notifying streams of changes: isAuthenticated: ${await _authStreamController.stream.last} => $isAuthenticated");
    _authStreamController.add(isAuthenticated); // Notify stream
    await updateUserDetails();
    notifyListeners();
  }

  Future<void> _saveToken(String token) async {
    print("saving token: $token");
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString('refresh_token', token);
    _refreshToken = token;
    _authStreamController.add(true); // Notify authenticated
  }

  // Delete token (logout)
  Future<void> _deleteToken() async {
    _accessToken = null;
    final prefs = await SharedPreferences.getInstance();
    await prefs.remove('refresh_token');
    _refreshToken = null;
    _authStreamController.add(false); // Notify unauthenticated
  }

  Future<void> _refreshAccessToken() async {
    try {
      final Map<String, String> headers = {
        'Content-Type': 'application/json',
      };
      if (!kIsWeb) {
        headers['cookie'] = 'refresh=$_refreshToken';
      }
      final response = await client.post(
        Uri.parse('$baseUrl/auth/refresh'),
        headers: headers,
      );
      final data = jsonDecode(response.body);
      if (response.statusCode == 200) {
        _accessToken = data['access_token'];
      } else {
        throw ApiError("refresh_failed", "Failed to refresh access token");
      }
    } catch (e) {
      rethrow;
    }
  }

  Future<(dynamic, int, Map<String, String>)> apiRequest(
    String endpoint, 
    {
      String? method,
      dynamic body,
      String? query,
      Map<String, String>? headers,
    }
  ) async {
    final request = http.Request(
      method ?? "GET",
      Uri.parse('$baseUrl/$endpoint${query ?? ""}'),
    );
    if (headers != null) {
      for (final entry in headers.entries) {
        request.headers[entry.key] = entry.value;
      }
    }
    if (body != null) {
      request.headers['content-type'] = 'application/json';
      request.body = jsonEncode(body);
    }
    try {
      final response = await client.send(request);
      final body = await response.stream.bytesToString();
      if (response.statusCode >= 500) {
        throw ApiError("server_error", "Server error: ${response.statusCode}");
      }
      final data = jsonDecode(body);
      final headers = response.headers;
      return (data, response.statusCode, headers);
    } catch (e, s) {
      // TODO: add error handling
      if (e is !ApiError) {
        print(e);
        print(s);
      }
      rethrow;
    }
  }

  Future<void> login(String user, String password) async {
    _isLoading = true;

    try {
      final (data, status, headers) = await apiRequest(
        'auth/login',
        method: 'POST',
        body: {
          'username': user,
          'password': password,
        },
      );
      if (status == 200) {
        _accessToken = data['access_token'];
        if (!kIsWeb) {
          final refresh = headers['set-cookie']!.split(';')[0].split('=')[1];
          print("saving token: $refresh");
          await _saveToken(refresh);
        }
        await updateUserDetails();
        notifyListeners();
      } else {
        throw ApiError.fromData(data);
      }
    } catch (e) {
      rethrow;
    } finally {
      _isLoading = false;
    }
  }

  Future<void> resetPassword(String password) async {
    await api("user/$username", method: 'PATCH', data: {
      'password': password,
    });
    await updateUserDetails();
    notifyListeners();
  }

  Future<void> updateUserDetails() async {
    if (!isAuthenticated) {
      username = null;
      isAdmin = null;
      passwordReset = null;
    } else {
      final me = await api("user/me");
      username = me['username'];
      isAdmin = me['is_admin'];
      passwordReset = me['password_reset'];
    }
  }

  Future<void> register(String email, String password) async {
  }

  // Logout
  Future<void> logout() async {
    await _deleteToken();
    await updateUserDetails();
    notifyListeners();
  }

  // Api call wrapper: returns the dynamic data on success or an error on failure
  Future<(dynamic, Map<String, String>)> apiAndHeaders(String endpoint, {String? method, dynamic data, String? query}) async {
    final body = data;
    try {
      final (data, status, headers) = await apiRequest(
        endpoint,
        query: query,
        method: method,
        body: body,
        headers: {
          'authorization': "Bearer $_accessToken",
          'content-type': 'application/json',
        }
      );
      if (status == 200) {
        return (data, headers);
      } else if (status == 401 && ["invalid_access_token", "expired_access_token", "missing_access_token"].contains(data['error'])) {
        try {
          await _refreshAccessToken();
        } catch (e) {
          if (e is ApiError) {
            if (e.kind == "refresh_failed") {
              await logout();
              rethrow;
            }
          } else {
            throw ApiError("refresh_failed", "Failed to refresh access token: $e");
          }
        }
        return await apiAndHeaders(endpoint, method: method, data: body);
      } else if (status == 401 && data['error'] == "expired_password") {
        passwordReset = true;
        notifyListeners();
        throw ApiError.fromData(data);
      } else {
        throw ApiError.fromData(data);
      }
    } catch (e) {
      rethrow;
    }
  }

  Future<dynamic> api(String endpoint, {String? method, dynamic data, String? query}) async {
    final response = await apiAndHeaders(endpoint, method: method, data: data, query: query);
    return response.$1;
  }

  Future<Media> getVideo(String video) async {
    String token;
    if (!_videoTokens.containsKey(video) || _videoTokens[video]!.expires.isBefore(DateTime.now())) {
      // get token then return media
      token = await api("video/$video/token").then((res) {
        final vtoken = VideoToken(res['token'], DateTime.parse(res['expires']));
        _videoTokens[video] = vtoken;
        return vtoken.token;
      });
    } else {
      token = _videoTokens[video]!.token;
    }
    return Media(
      '$baseUrl/media/$token',
    );
  }

  Future<dynamic> uploadVideos(String game, List<PlatformFile> files, Map<String, String> names, Map<String, bool> publics, Function(int, int) onProgress) async {
    final responses = [];
    for (var i = 0; i < files.length; i++) {
      final form = FormData();
      form.fields.add(MapEntry("game", game));
      form.fields.add(MapEntry("files.0.name", names[files[i].name] ?? files[i].name));
      form.fields.add(MapEntry("files.0.public", (publics[files[i].name] ?? false).toString()));
      form.files.add(MapEntry(
        "files.0.file",
        MultipartFile.fromStream(
          () => files[i].readStream!,
          files[i].size,
          filename: files[i].name,
        ),
      ));

      final response = await Dio().post(
        "$baseUrl/video/upload",
        data: form,
        onSendProgress: (int sent, int total) {
          onProgress(sent, total);
        },
        options: Options(
          headers: {
            "authorization": "Bearer $_accessToken",
          }
        ),
      );
      try {
        final data = response.data;
        if (response.statusCode == 200) {
          responses.add(response.data[0]);
        } else {
          throw "API Error: ${data['error']}: ${data['message']}";
        }
      } catch (e) {
        rethrow;
      }
    }
    return responses;
  }
}

class VideoToken {
  final String token;
  final DateTime expires;

  const VideoToken(this.token, this.expires);
}

const int CHUNK_SIZE = 1024 * 1024;

Stream<List<int>> streamWrapper(Stream<List<int>> stream) async* {
  final ChunkedStreamReader<int> reader = ChunkedStreamReader(stream);
  while (true) {
    final chunk = await reader.readChunk(CHUNK_SIZE);
    yield chunk;
    if (chunk.length < CHUNK_SIZE) {
      break;
    }
  }
}

class ApiError {
  final String? kind;
  final String message;

  ApiError(this.kind, this.message);

  factory ApiError.fromData(dynamic data) => ApiError(data['error'], data['message']);

  @override
  String toString() => message;
}
