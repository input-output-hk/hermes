import 'package:dashboard/l10n/l10n.dart';
import 'package:flutter/material.dart';

final class App extends StatelessWidget {
  const App({super.key});

  @override
  Widget build(BuildContext context) {
    return const MaterialApp(
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: Scaffold(
        body: Center(
          child: Text(
            'Hermes Dashboard',
            style: TextStyle(
              fontSize: 48,
            ),
          ),
        ),
      ),
    );
  }
}
