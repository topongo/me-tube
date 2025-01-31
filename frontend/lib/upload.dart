import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import 'auth.dart';

class UploadScreen extends StatefulWidget {
  @override
  _UploadScreenState createState() => _UploadScreenState();
}

class _UploadScreenState extends State<UploadScreen> {
  FilePickerResult? _files;

  @override
  Widget build(BuildContext context) {
    final auth = Provider.of<AuthService>(context);
    return Scaffold(
      appBar: AppBar(
        title: const Text('Upload'),
      ),
      body: Center(
        child: Form(
          child: Column(
            children: [
              ElevatedButton(onPressed: () {
                FilePicker.platform.pickFiles(allowMultiple: true).then((result) {
                  if (result != null) {
                    setState(() => _files = result);
                  }
                });
              }, child: const Text('Select Files')),
              ListView.builder(
                shrinkWrap: true,
                itemCount: _files?.files.length ?? 0,
                itemBuilder: (context, index) {
                  final file = _files!.files[index];
                  return ListTile(
                    title: Text(file.name),
                    subtitle: Text('${file.size} bytes'),
                    trailing: IconButton(
                      icon: Icon(Icons.delete),
                      onPressed: () {
                        setState(() => _files!.files.removeAt(index));
                      },
                    ),
                  );
                }
              )
            ]
          )
        )
      )
    );
  }
}
