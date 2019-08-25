import React from 'react';
import AppBar from '@material-ui/core/AppBar';
import Toolbar from '@material-ui/core/Toolbar';
import Typography from '@material-ui/core/Typography';
import { navigation } from '../navigations'
import { LoginLink, SectionLink } from './NavigationsControls'
import { styles } from './styles';
import { strings } from '../../../localization/localization.js';

export function NavigationBarExtended(props) {
    const { classes } = props;
    return (
        <AppBar style={styles.appBar} position="static" color="default" className={classes.appBar}>
            <Toolbar>
                {/* just a placeholder */}
                <Typography variant="h6" color="inherit" noWrap className={classes.toolbarTitle} />

                <SectionLink to={navigation.pricing} label={strings.pricing} />
                <LoginLink to={navigation.singIn} label={strings.login} />
            </Toolbar>
        </AppBar>
    );
}

export function NavigationBarSimple(props) {
    const { classes } = props;
    return (
        <AppBar style={styles.appBar} position="static" color="default" className={classes.appBar}>
            <Toolbar />
        </AppBar>
    );
}