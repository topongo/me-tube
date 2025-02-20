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
  Media? media;
  late final player = Player();
  late final controller = VideoController(player);
  bool _loaded = false;

  @override
  void initState() {
    super.initState();
    final auth = Provider.of<AuthService>(context, listen: false);
    if (player.platform is libmpvPlayer) {
      // (player.platform as dynamic).setProperty('tls-cert-file', 'assets/cert.pem').then((_) {
        // _loadVideo(auth);
        // setState(() => _loaded = true);
      // });
    } else {
      _loadVideo(auth);
    }
  }

  Future<void> _loadVideo(AuthService auth) {
      return auth.getVideo(widget.video).then((m) {
        media = m;
        player.open(media!);
        setState(() => _loaded = true);
      });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text('Video'),
        elevation: 0,
        backgroundColor: Colors.transparent,
        actions: [
          IconButton(
            icon: Icon(Icons.favorite),
            onPressed: () {
              final auth = Provider.of<AuthService>(context, listen: false);
              auth.api("video/${widget.video}/favorite", method: "POST").then((_) {
                ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('Video favorited')));
              });
            }
          )
        ]
      ),
      extendBodyBehindAppBar: true,
      extendBody: true,
      body: LayoutBuilder(
        builder: (context, constraints) {
          return !_loaded ? const CircularProgressIndicator() : Video(controller: controller);
        }
      )
    );
  }

  @override
  dispose() {
    player.dispose();
    super.dispose();
  }
}
