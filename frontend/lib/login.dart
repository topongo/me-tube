import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import 'auth.dart';

class LoginScreen extends StatefulWidget {
  @override
  _LoginScreenState createState() => _LoginScreenState();
}

class _LoginScreenState extends State<LoginScreen> {
  final _formKey = GlobalKey<FormState>();
  final _usernameController = TextEditingController();
  final _passwordController = TextEditingController();
  bool _isLoading = false;

  Future<void> _signIn() async {
    if (!_formKey.currentState!.validate()) return;

    setState(() => _isLoading = true);
    try {
      // retriever AuthService from Provider
      final authService = Provider.of<AuthService>(context, listen: false);
      await authService.login(
        _usernameController.text,
        _passwordController.text,
      );
    } catch (e) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text("$e")),
      );
    } finally {
      setState(() => _isLoading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: Text('Login')),
      body: Padding(
        padding: EdgeInsets.all(20),
        child: Form(
          key: _formKey,
          child: Column(
            children: [
              TextFormField(
                controller: _usernameController,
                autofillHints: [AutofillHints.username],
                decoration: InputDecoration(labelText: 'Username'),
                keyboardType: TextInputType.text,
                validator: (value) =>
                    value!.isEmpty ? 'Enter your username' : null,
                onEditingComplete: _signIn,
                autofocus: true,
              ),
              TextFormField(
                controller: _passwordController,
                autofillHints: [AutofillHints.password],
                decoration: InputDecoration(labelText: 'Password'),
                obscureText: true,
                validator: (value) =>
                    value!.isEmpty ? 'Enter your password' : null,
                onEditingComplete: _signIn,
              ),
              SizedBox(height: 20),
              _isLoading
                  ? CircularProgressIndicator()
                  : ElevatedButton(
                      onPressed: _signIn,
                      child: Text('Sign In'),
                    ),
              TextButton(
                onPressed: () {
                  // Add navigation to sign-up screen
                },
                child: Text('Create Account'),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
