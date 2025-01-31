import 'dart:convert';
import 'data/authentication.dart';
import 'package:http/http.dart' as http;

class Api {
  static const String baseUrl = 'http://127.0.0.1:8001';

  static Future<LoginResponse> login(UserForm form) async {
    final response = await http.post(
      Uri.parse("$baseUrl/login"),
      headers: <String, String>{
        'content-type': 'application/json',
      }
    );
    return jsonDecode(response.body);
  }
}
