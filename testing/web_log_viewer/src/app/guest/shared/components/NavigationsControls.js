import React from 'react';
import Button from '@material-ui/core/Button';
import ArrowDropDown from '@material-ui/icons/ArrowDropDown';
import { Link } from 'react-router-dom';
import { styles } from './styles';

export class ScrollDownButton extends React.Component {
    render() {

        const navigationTarget = this.props.navigationTarget
        const buttonStyle = styles.button
        const linkStyle = styles.link

        return (
            <a href={navigationTarget} style={linkStyle}>
                <Button style={buttonStyle} variant="fab" color="inherit" aria-label="ArrowDropDown" >
                    <ArrowDropDown />
                </Button>
            </a>);
    }
}

export class SectionLink extends React.Component {
    render() {

        const navigationTarget = this.props.to
        const linkLabel = this.props.label
        const buttonStyle = styles.button
        const linkStyle = styles.link

        return (
            <a href={navigationTarget} style={linkStyle}>
                <Button style={buttonStyle}>{linkLabel}</Button>
            </a>);
    }
}

export class LoginLink extends React.Component {
    render() {
        
        const navigationTarget = this.props.to
        const linkLabel = this.props.label
        const linkStyle = styles.link

        return (
            <Link to={navigationTarget} style={linkStyle}>
                <Button color="primary" variant="outlined">{linkLabel}</Button>
            </Link>);
    }
}
