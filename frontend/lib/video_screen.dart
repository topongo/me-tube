import 'package:flutter/material.dart';
import 'package:media_kit/media_kit.dart';
import 'package:media_kit_video/media_kit_video.dart';
import 'package:provider/provider.dart';
import 'auth.dart';

class VideoScreen extends StatefulWidget {
  final String video;
  const VideoScreen({super.key, required this.video});

  @override
  _VideoScreenState createState() => _VideoScreenState();
}

class _VideoScreenState extends State<VideoScreen> {
  late Media media;
  late final player = Player();
  late final controller = VideoController(player);
  bool _loaded = false;

  @override
  void initState() {
    super.initState();
    if (player.platform is libmpvPlayer) {
      (player.platform as dynamic).setProperty('tls-cert-file', 'assets/cert.pem').then((_) {
        setState(() => _loaded = true);
      });
    }
    final auth = Provider.of<AuthService>(context, listen: false);
    final media = auth.getVideo(widget.video);
    player.open(media);
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: LayoutBuilder(
        builder: (context, constraints) {
          return !_loaded ? const CircularProgressIndicator() : Video(controller: controller);
        }
      )
    );
  }
}
