import 'dart:io';

import 'package:flutter/material.dart';
import 'auth.dart';
import 'home.dart';
import 'package:media_kit/media_kit.dart';
import 'login.dart';
import 'package:provider/provider.dart';
import 'package:flutter/services.dart' show rootBundle;

late final List<int> certificate;

void main() async {
  MediaKit.ensureInitialized();
  WidgetsFlutterBinding.ensureInitialized();

  final stream = await rootBundle.load('assets/cert.pem');
  certificate = stream.buffer.asUint8List();

  runApp(MyApp());
}

class MyApp extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return ChangeNotifierProvider(
      create: (context) => AuthService(),
      child: MaterialApp(
        title: 'Auth Demo',
        // Show LoginScreen or HomeScreen based on auth state
        home: AuthWrapper(),
      ),
    );
  }
}

class AuthWrapper extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final authService = Provider.of<AuthService>(context);
    print("isAuthenticated: ${authService.isAuthenticated}");
    return Consumer<AuthService>(
      builder: (context, authService, child) {
        if (!authService.isAuthenticated) {
          return LoginScreen();
        } else {
          return HomeScreen();
        }
      }
    );
  }
}

