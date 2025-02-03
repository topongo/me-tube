// auth_service.dart
import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/foundation.dart';
import 'main.dart';
import 'multipart.dart';
import 'package:http/http.dart' as http;
import 'package:http/io_client.dart';
import 'package:media_kit/media_kit.dart';
import 'package:shared_preferences/shared_preferences.dart';

class AuthService with ChangeNotifier {
  static String baseUrl = 'https://metube.prabo.org/api';
  String? _refreshToken;
  String? _accessToken;
  bool _isLoading = false;
  String? _error;
  late http.Client client;
  String? username;

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
    if (!kIsWeb) {
      final context = SecurityContext.defaultContext;
      context.setTrustedCertificatesBytes(certificate);
      client = IOClient(HttpClient(context: context));
    } else {
      client = http.Client();
    }
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
        print("Refresh failed: ${response.body}");
        _error = 'Refresh failed: ${response.body}';
      }
    } catch (e) {
      rethrow;
    }
  }

  // Example: Login via API
  Future<void> login(String user, String password) async {
    _isLoading = true;
    // notifyListeners();

    try {
      final response = await client.post(
        Uri.parse('$baseUrl/auth/login'),
        body: jsonEncode({'username': user, 'password': password}),
        headers: {'Content-Type': 'application/json'},
      );

      if (response.statusCode == 200) {
        final data = jsonDecode(response.body);
        _accessToken = data['access_token'];
        username = user;
        _error = null;
        if (!kIsWeb) {
          final refresh = response.headers['set-cookie']!.split(';')[0].split('=')[1];
          print("saving token: $refresh");
          await _saveToken(refresh);
        }
        notifyListeners();
      } else {
        _error = 'Login failed: ${response.body}';
      }
    } catch (e) {
      rethrow;
    } finally {
      _isLoading = false;
    }
  }

  Future<void> register(String email, String password) async {
    // Similar structure to login()
  }

  // Logout
  Future<void> logout() async {
    await _deleteToken();
    notifyListeners();
  }

  // Api call wrapper: returns the dynamic data on success or an error on failure
  Future<dynamic> api(String endpoint, {String? method, dynamic data}) async {
    final request = http.Request(method ?? "GET", Uri.parse('$baseUrl/$endpoint'));
    request.headers['content-type'] = "application/json";
    request.headers['authorization'] = "Bearer $_accessToken";
    try {
      final response = await client.send(request);
      final body = await response.stream.bytesToString();
      final data = jsonDecode(body);
      if (response.statusCode == 200) {
        _error = null;
        return data;
      } else if (response.statusCode == 401 && ["invalid_access_token", "expired_access_token", "missing_access_token"].contains(data['error'])) {
        _error = null;
        await _refreshAccessToken();
        if(_error != null) {
          await logout();
          throw _error!;
        }
        return await api(endpoint, method: method, data: data);
      } else {
        throw 'API error: $data';
      }
    } catch (e) {
      rethrow;
    }
  }

  Media getVideo(String video) {
    return Media(
      '$baseUrl/media/$video',
      httpHeaders: {'authorization': 'Bearer $_accessToken'},
    );
  }

  Future<dynamic> uploadVideos(String game, List<PlatformFile> files, Map<String, String> names, Function(int, int) onProgress) async {
    final request = StreamMultipartRequest(
      "POST", 
      Uri.parse("$baseUrl/video/upload"),
      onProgress: onProgress,
    );
    request.fields["game"] = game;
    for (var i = 0; i < files.length; i++) {
      request.files.add(http.MultipartFile("files[$i].file", files[i].readStream!, files[i].size));
      request.fields["files[$i].name"] = names[files[i].name] ?? files[i].name;
    }
    // force token refresh: we don't want the request to fail and all the data stream ruined.
    await _refreshAccessToken();
    request.headers["authorization"] = "Bearer $_accessToken";
    final response = await client.send(request);
    try {
      final data = jsonDecode(await response.stream.bytesToString());
      if (response.statusCode == 200) {
        return data;
      } else {
        throw data['error'];
      }
    } catch (e) {
      rethrow;
    }
  }
}
