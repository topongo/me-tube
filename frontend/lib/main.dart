import 'dart:io';

import 'package:MeTube/password_reset.dart';
import 'package:async/async.dart';
import 'package:flutter/material.dart';
import 'auth.dart';
import 'home.dart';
import 'package:media_kit/media_kit.dart';
import 'login.dart';
import 'package:provider/provider.dart';
import 'package:flutter/services.dart' show rootBundle;

late final List<int> certificate;

void main() async {
  // print("open file");
  // final f = File('/home/topongo/downloads/Severance.S02E03.1080p.HEVC.x265-MeGusta.mkv');
  // print("create stream");
  // final stream = f.openRead();
  // print("create chunked stream");
  // final chunkedStream = ChunkedStreamReader(stream);
  // print("reading first chunk");
  // final chunk = await chunkedStream.readChunk(100000000000);
  // print(chunk.length);
  // exit(0);
  

  MediaKit.ensureInitialized();
  WidgetsFlutterBinding.ensureInitialized();

  // final stream = await rootBundle.load('assets/cert.pem');
  // certificate = stream.buffer.asUint8List();

  runApp(MyApp());
}

class MyApp extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return ChangeNotifierProvider(
      create: (context) => AuthService(),
      child: MaterialApp(
        theme: ThemeData(
          brightness: Brightness.dark,
          primaryColorDark: Color.fromARGB(0xff, 0x50, 0x50, 0xff),
        ),
        title: 'MeTube',
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
        } else if (authService.isAuthenticated && authService.passwordReset == true) {
          return PasswordResetScreen();
        } else {
          return HomeScreen();
        }
      }
    );
  }
}

