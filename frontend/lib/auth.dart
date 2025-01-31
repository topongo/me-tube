// auth_service.dart
import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:frontend2/main.dart';
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
  late IOClient client;
  String? username;

  // Getters
  bool get isAuthenticated => _refreshToken != null;
  bool get isLoading => _isLoading;
  String? get error => _error;

  // Stream for auth state changes
  final _authStreamController = StreamController<bool>();
  Stream<bool> get authState => _authStreamController.stream;

  // Initialize from local storage (e.g., token persistence)
  AuthService() {
    final context = SecurityContext.defaultContext;
    context.setTrustedCertificatesBytes(certificate);
    client = IOClient(HttpClient(context: context));
    _loadToken();
  }

  // Load token from SharedPreferences
  Future<void> _loadToken() async {
    final prefs = await SharedPreferences.getInstance();
    _refreshToken = prefs.getString('refresh_token');
    if (_refreshToken != null) {
      await refreshToken();
    }
    _authStreamController.add(isAuthenticated); // Notify stream
    notifyListeners();
  }

  // Save token to SharedPreferences
  Future<void> _saveToken(String token) async {
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

  Future<void> refreshToken() async {
    try {
      final response = await client.post(
        Uri.parse('$baseUrl/auth/refresh'),
        headers: {'Cookie': 'refresh=$_refreshToken'},
      );
      final data = jsonDecode(response.body);
      if (response.statusCode == 200) {
        _accessToken = data['access_token'];
      } else {
        _error = 'Refresh failed: ${response.body}';
      }
    } catch (e) {
      rethrow;
    }
  }

  // Example: Login via API
  Future<void> login(String user, String password) async {
    _isLoading = true;
    notifyListeners();

    try {
      final response = await client.post(
        Uri.parse('$baseUrl/auth/login'),
        body: jsonEncode({'username': user, 'password': password}),
        headers: {'Content-Type': 'application/json'},
      );

      if (response.statusCode == 200) {
        final data = jsonDecode(response.body);
        _accessToken = data['access_token'];
        final refresh = response.headers['set-cookie']!.split(';')[0].split('=')[1];
        username = user;
        _error = null;
        await _saveToken(refresh);
      } else {
        _error = 'Login failed: ${response.body}';
      }
    } catch (e) {
      rethrow;
    } finally {
      _isLoading = false;
      notifyListeners();
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
        await refreshToken();
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
      httpHeaders: {'authorization': 'Bearer $_accessToken', 'range': 'bytes=0-'},
    );
  }
}
