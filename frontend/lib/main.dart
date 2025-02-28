import 'dart:io';
import 'package:MeTube/password_reset.dart';
import 'package:MeTube/video_screen.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'auth.dart';
import 'home.dart';
import 'package:media_kit/media_kit.dart';
import 'login.dart';
import 'package:provider/provider.dart';

late final String apiBaseUrl;

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

  // get apiBaseUrl from env variable
  if (!kIsWeb) {
    if (kDebugMode) {
      apiBaseUrl = Platform.environment["API_BASE_URL"] ?? "http://127.0.0.1:8000/api";
    } else {
      throw Exception("native app isn't meant to be used in release mode");
    }
  } else {
    apiBaseUrl = "/api";
  }
  

  MediaKit.ensureInitialized();
  WidgetsFlutterBinding.ensureInitialized();

  // final stream = await rootBundle.load('assets/cert.pem');
  // certificate = stream.buffer.asUint8List();

  final authService = AuthService();

  String initialRoute = "/login";
  if (await authService.init()) {
    debugPrint("Authenticated");
    if (authService.passwordReset == true) {
      debugPrint("Password reset required");
      initialRoute = "/password_reset";
    } else {
      initialRoute = "/";
    }
  } else {
    debugPrint("Not authenticated");
    initialRoute = "/login";
  }

  runApp(MeTube(
    initialRoute: initialRoute,
    authService: authService,
  ));
}

class MeTube extends StatelessWidget {
  final String initialRoute;
  final AuthService authService;

  MeTube({required this.initialRoute, required this.authService});

  @override
  Widget build(BuildContext context) {
    debugPrint("rendering initial route: $initialRoute");
    return ChangeNotifierProvider(
      create: (context) => authService,
      child: MaterialApp(
        theme: ThemeData(
          brightness: Brightness.dark,
          primaryColorDark: Color.fromARGB(0xff, 0x50, 0x50, 0xff),
        ),
        title: 'MeTube',
        // Show LoginScreen or HomeScreen based on auth state
        routes: {
          '/': (c) => HomeScreen(),
          '/login': (c) => LoginScreen(),
          '/password_reset': (c) => PasswordResetScreen(),
        },
        initialRoute: initialRoute,
        onGenerateInitialRoutes: (String initialRouteName) {
          return [
            MaterialPageRoute(
              builder: (context) {
                return switch (initialRoute) {
                  "/" => HomeScreen(),
                  "/login" => LoginScreen(),
                  "/password_reset" => PasswordResetScreen(),
                  _ => LoginScreen(),
                };
              },
            ),
          ];
        },
        onGenerateRoute: (settings) {
          if (settings.name?.startsWith("/watch") == true) {
            final videoId = settings.name!.split("/").last;
            return MaterialPageRoute(builder: (context) {
              return VideoScreen(video: videoId);
            });
          } else {
            throw Exception("Unknown route: ${settings.name}");
          }
        }
      ),
    );
  }
}

class AuthWrapper extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final authService = Provider.of<AuthService>(context);
    // print("isAuthenticated: ${authService.isAuthenticated}");
    return Consumer<AuthService>(
      builder: (context, authService, child) {
        return HomeScreen();
        // if (authService.isLoading) {
        //   return Scaffold(
        //     body: Center(
        //       child: Column(children: [
        //         Text("Loading..."),
        //         const SizedBox(height: 20),
        //         const CircularProgressIndicator(),
        //       ]),
        //     )
        //   );
        // } else {
        //   if (!authService.isAuthenticated) {
        //     return LoginScreen();
        //   } else if (authService.isAuthenticated && authService.passwordReset == true) {
        //     return PasswordResetScreen();
        //   } else {
        //     return HomeScreen();
        //   }
        // }
      }
    );
  }
}

